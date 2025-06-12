//! Db-Sync data source implementation that queries Cardano block information
use crate::{
	DataSourceError::*,
	data_sources::read_mc_epoch_config,
	db_model::{self, Block, BlockNumber, SlotNumber},
};
use chrono::{DateTime, NaiveDateTime, TimeDelta};
use derive_new::new;
use figment::{Figment, providers::Env};
use log::{debug, info};
use serde::Deserialize;
use sidechain_domain::mainchain_epoch::{MainchainEpochConfig, MainchainEpochDerivation};
use sidechain_domain::*;
use sp_timestamp::Timestamp;
use sqlx::PgPool;
use std::{
	error::Error,
	sync::{Arc, Mutex},
};

#[cfg(test)]
mod tests;

/// Db-Sync data source that queries Cardano block information
///
/// This data source does not implement any data source interface used by one of the
/// Partner Chain toolkit's features, but is used internally by other data sources
/// that require access to Cardano block data
#[allow(clippy::too_many_arguments)]
#[derive(new)]
pub struct BlockDataSourceImpl {
	/// Postgres connection pool
	pool: PgPool,
	/// Cardano security parameter
	///
	/// This parameter controls how many confirmations (blocks on top) are required by
	/// the Cardano node to consider a block to be stable. This is a network-wide parameter.
	security_parameter: u32,
	/// Minimal age of a block to be considered valid stable in relation to some given timestamp.
	/// Must be equal to `security parameter / active slot coefficient`.
	min_slot_boundary_as_seconds: TimeDelta,
	/// a characteristic of Ouroboros Praos and is equal to `3 * security parameter / active slot coefficient`
	max_slot_boundary_as_seconds: TimeDelta,
	/// Cardano main chain epoch configuration
	mainchain_epoch_config: MainchainEpochConfig,
	/// Additional offset applied when selecting the latest stable Cardano block
	///
	/// This parameter should be 0 by default and should only be increased to 1 in networks
	/// struggling with frequent block rejections due to Db-Sync or Cardano node lag.
	block_stability_margin: u32,
	/// Number of contiguous Cardano blocks to be cached by this data source
	cache_size: u16,
	/// Internal block cache
	stable_blocks_cache: Arc<Mutex<BlocksCache>>,
}

impl BlockDataSourceImpl {
	/// Returns the latest _unstable_ Cardano block from the Db-Sync database
	pub async fn get_latest_block_info(
		&self,
	) -> Result<MainchainBlock, Box<dyn std::error::Error + Send + Sync>> {
		db_model::get_latest_block_info(&self.pool)
			.await?
			.map(From::from)
			.ok_or(ExpectedDataNotFound("No latest block on chain.".to_string()).into())
	}

	/// Returns the latest _stable_ Cardano block from the Db-Sync database that is within
	/// acceptable bounds from `reference_timestamp`, accounting for the additional stability
	/// offset configured by [block_stability_margin][Self::block_stability_margin].
	pub async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		let reference_timestamp = BlockDataSourceImpl::timestamp_to_db_type(reference_timestamp)?;
		let latest = self.get_latest_block_info().await?;
		let offset = self.security_parameter + self.block_stability_margin;
		let stable = BlockNumber(latest.number.0.saturating_sub(offset));
		let block = self.get_latest_block(stable, reference_timestamp).await?;
		Ok(block.map(From::from))
	}

	/// Finds a block by its `hash` and verifies that it is stable in reference to `reference_timestamp`
	/// and returns its info
	pub async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		let reference_timestamp = BlockDataSourceImpl::timestamp_to_db_type(reference_timestamp)?;
		self.get_stable_block_by_hash(hash, reference_timestamp).await
	}

	/// Finds a block by its `hash` and returns its info
	pub async fn get_block_by_hash(
		&self,
		hash: McBlockHash,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		let from_cache = if let Ok(cache) = self.stable_blocks_cache.lock() {
			cache.find_by_hash(hash.clone())
		} else {
			None
		};
		let block_opt = match from_cache {
			Some(block) => Some(block),
			None => db_model::get_block_by_hash(&self.pool, hash).await?,
		};
		Ok(block_opt.map(From::from))
	}
}

/// Configuration for [BlockDataSourceImpl]
#[derive(Debug, Clone, Deserialize)]
pub struct DbSyncBlockDataSourceConfig {
	/// Cardano security parameter, ie. the number of confirmations needed to stabilize a block
	pub cardano_security_parameter: u32,
	/// Expected fraction of Cardano slots that will have a block produced
	///
	/// This value can be found in `shelley-genesis.json` file used by the Cardano node,
	/// example: `"activeSlotsCoeff": 0.05`.
	pub cardano_active_slots_coeff: f64,
	/// Additional offset applied when selecting the latest stable Cardano block
	///
	/// This parameter should be 0 by default and should only be increased to 1 in networks
	/// struggling with frequent block rejections due to Db-Sync or Cardano node lag.
	pub block_stability_margin: u32,
}

impl DbSyncBlockDataSourceConfig {
	/// Reads the config from environment
	pub fn from_env() -> std::result::Result<Self, Box<dyn Error + Send + Sync + 'static>> {
		let config: Self = Figment::new()
			.merge(Env::raw())
			.extract()
			.map_err(|e| format!("Failed to read block data source config: {e}"))?;
		info!("Using block data source configuration: {config:?}");
		Ok(config)
	}
}

impl BlockDataSourceImpl {
	/// Creates a new instance of [BlockDataSourceImpl], reading configuration from the environment.
	pub async fn new_from_env(
		pool: PgPool,
	) -> std::result::Result<Self, Box<dyn Error + Send + Sync + 'static>> {
		Ok(Self::from_config(
			pool,
			DbSyncBlockDataSourceConfig::from_env()?,
			&read_mc_epoch_config()?,
		))
	}

	/// Creates a new instance of [BlockDataSourceImpl], using passed configuration.
	pub fn from_config(
		pool: PgPool,
		DbSyncBlockDataSourceConfig {
			cardano_security_parameter,
			cardano_active_slots_coeff,
			block_stability_margin,
		}: DbSyncBlockDataSourceConfig,
		mc_epoch_config: &MainchainEpochConfig,
	) -> BlockDataSourceImpl {
		let k: f64 = cardano_security_parameter.into();
		let slot_duration: f64 = mc_epoch_config.slot_duration_millis.millis() as f64;
		let min_slot_boundary = (slot_duration * k / cardano_active_slots_coeff).round() as i64;
		let max_slot_boundary = 3 * min_slot_boundary;
		let cache_size = 100;
		BlockDataSourceImpl::new(
			pool,
			cardano_security_parameter,
			TimeDelta::milliseconds(min_slot_boundary),
			TimeDelta::milliseconds(max_slot_boundary),
			mc_epoch_config.clone(),
			block_stability_margin,
			cache_size,
			BlocksCache::new_arc_mutex(),
		)
	}
	async fn get_latest_block(
		&self,
		max_block: BlockNumber,
		reference_timestamp: NaiveDateTime,
	) -> Result<Option<Block>, Box<dyn std::error::Error + Send + Sync>> {
		let min_time = self.min_block_allowed_time(reference_timestamp);
		let min_slot = self.date_time_to_slot(min_time)?;
		let max_time = self.max_allowed_block_time(reference_timestamp);
		let max_slot = self.date_time_to_slot(max_time)?;
		Ok(db_model::get_highest_block(
			&self.pool, max_block, min_time, min_slot, max_time, max_slot,
		)
		.await?)
	}

	fn min_block_allowed_time(&self, reference_timestamp: NaiveDateTime) -> NaiveDateTime {
		reference_timestamp - self.max_slot_boundary_as_seconds
	}

	fn max_allowed_block_time(&self, reference_timestamp: NaiveDateTime) -> NaiveDateTime {
		reference_timestamp - self.min_slot_boundary_as_seconds
	}

	/// Rules for block selection and verification mandates that timestamp of the block
	/// falls in a given range, calculated from the reference timestamp, which is either
	/// PC current time or PC block timestamp.
	fn is_block_time_valid(&self, block: &Block, timestamp: NaiveDateTime) -> bool {
		self.min_block_allowed_time(timestamp) <= block.time
			&& block.time <= self.max_allowed_block_time(timestamp)
	}

	async fn get_stable_block_by_hash(
		&self,
		hash: McBlockHash,
		reference_timestamp: NaiveDateTime,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		if let Some(block) =
			self.get_stable_block_by_hash_from_cache(hash.clone(), reference_timestamp)
		{
			debug!("Block by hash: {hash} found in cache.");
			Ok(Some(From::from(block)))
		} else {
			debug!("Block by hash: {hash}, not found in cache, serving from database.");
			if let Some(block_by_hash) =
				self.get_stable_block_by_hash_from_db(hash, reference_timestamp).await?
			{
				self.fill_cache(&block_by_hash).await?;
				Ok(Some(MainchainBlock::from(block_by_hash)))
			} else {
				Ok(None)
			}
		}
	}

	fn get_stable_block_by_hash_from_cache(
		&self,
		hash: McBlockHash,
		reference_timestamp: NaiveDateTime,
	) -> Option<Block> {
		if let Ok(cache) = self.stable_blocks_cache.lock() {
			cache
				.find_by_hash(hash)
				.filter(|block| self.is_block_time_valid(block, reference_timestamp))
		} else {
			None
		}
	}

	/// Returns block by given hash from the cache if it is valid in reference to given timestamp
	async fn get_stable_block_by_hash_from_db(
		&self,
		hash: McBlockHash,
		reference_timestamp: NaiveDateTime,
	) -> Result<Option<Block>, Box<dyn std::error::Error + Send + Sync>> {
		let block = db_model::get_block_by_hash(&self.pool, hash).await?;
		let latest_block = db_model::get_latest_block_info(&self.pool).await?;
		Ok(block
			.zip(latest_block)
			.filter(|(block, latest_block)| {
				block.block_no.0 + self.security_parameter <= latest_block.block_no.0
					&& self.is_block_time_valid(block, reference_timestamp)
			})
			.map(|(block, _)| block))
	}

	/// Caches stable blocks for lookup by hash.
	async fn fill_cache(
		&self,
		from_block: &Block,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		let from_block_no = from_block.block_no;
		let size = u32::from(self.cache_size);
		let latest_block =
			db_model::get_latest_block_info(&self.pool)
				.await?
				.ok_or(InternalDataSourceError(
					"No latest block when filling the caches.".to_string(),
				))?;
		let latest_block_num = latest_block.block_no.0;
		let stable_block_num = latest_block_num.saturating_sub(self.security_parameter);

		let to_block_no = BlockNumber(from_block_no.0.saturating_add(size).min(stable_block_num));
		let blocks = if to_block_no.0 > from_block_no.0 {
			db_model::get_blocks_by_numbers(&self.pool, from_block_no, to_block_no).await?
		} else {
			vec![from_block.clone()]
		};

		if let Ok(mut cache) = self.stable_blocks_cache.lock() {
			cache.update(blocks);
			debug!("Cached blocks {} to {} for by hash lookups.", from_block_no.0, to_block_no.0);
		}
		Ok(())
	}

	fn date_time_to_slot(
		&self,
		dt: NaiveDateTime,
	) -> Result<SlotNumber, Box<dyn std::error::Error + Send + Sync>> {
		let millis: u64 = dt
			.and_utc()
			.timestamp_millis()
			.try_into()
			.map_err(|_| BadRequest(format!("Datetime out of range: {dt:?}")))?;
		let ts = sidechain_domain::mainchain_epoch::Timestamp::from_unix_millis(millis);
		let slot = self
			.mainchain_epoch_config
			.timestamp_to_mainchain_slot_number(ts)
			.unwrap_or(self.mainchain_epoch_config.first_slot_number);
		Ok(SlotNumber(slot))
	}

	fn timestamp_to_db_type(
		timestamp: Timestamp,
	) -> Result<NaiveDateTime, Box<dyn std::error::Error + Send + Sync>> {
		let millis: Option<i64> = timestamp.as_millis().try_into().ok();
		let dt = millis
			.and_then(DateTime::from_timestamp_millis)
			.ok_or(BadRequest(format!("Timestamp out of range: {timestamp:?}")))?;
		Ok(NaiveDateTime::new(dt.date_naive(), dt.time()))
	}
}

/// Helper structure for caching stable blocks.
#[derive(new)]
pub(crate) struct BlocksCache {
	/// Continuous main chain blocks. All blocks should be stable. Used to query by hash.
	#[new(default)]
	from_last_by_hash: Vec<Block>,
}

impl BlocksCache {
	fn find_by_hash(&self, hash: McBlockHash) -> Option<Block> {
		self.from_last_by_hash.iter().find(|b| b.hash == hash.0).cloned()
	}

	pub fn update(&mut self, from_last_by_hash: Vec<Block>) {
		self.from_last_by_hash = from_last_by_hash;
	}

	pub fn new_arc_mutex() -> Arc<Mutex<Self>> {
		Arc::new(Mutex::new(Self::new()))
	}
}
