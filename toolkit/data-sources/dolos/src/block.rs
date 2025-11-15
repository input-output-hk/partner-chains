use crate::{
	DataSourceError, Result,
	client::{MiniBFClient, api::MiniBFApi, conversions::from_block_content},
	read_mc_epoch_config,
};
use chrono::{DateTime, NaiveDateTime, TimeDelta};
use derive_new::new;
use figment::{Figment, providers::Env};
use log::{debug, info};
use serde::Deserialize;
use sidechain_domain::mainchain_epoch::{MainchainEpochConfig, MainchainEpochDerivation};
use sidechain_domain::*;
use sp_timestamp::Timestamp;
use std::{
	error::Error,
	sync::{Arc, Mutex},
};

#[derive(new)]
pub struct BlockDataSourceImpl {
	/// MiniBF client
	client: MiniBFClient,
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
	/// This parameter should be 1 by default.
	block_stability_margin: u32,
	/// Number of contiguous Cardano blocks to be cached by this data source
	cache_size: u16,
	/// Internal block cache
	stable_blocks_cache: Arc<Mutex<BlocksCache>>,
}

impl BlockDataSourceImpl {
	/// Returns the latest _unstable_ Cardano block from Dolos
	pub async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
		self.client
			.blocks_latest()
			.await
			.map_err(|e| {
				DataSourceError::ExpectedDataNotFound(format!("No latest block on chain. {e}",))
					.into()
			})
			.and_then(from_block_content)
	}

	/// Returns the latest _stable_ Cardano block from Dolos that is within
	/// acceptable bounds from `reference_timestamp`, accounting for the additional stability
	/// offset configured by [block_stability_margin][Self::block_stability_margin].
	pub async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>> {
		let reference_timestamp = BlockDataSourceImpl::timestamp_to_db_type(reference_timestamp)?;
		let latest = self.get_latest_block_info().await?;
		let offset = self.security_parameter + self.block_stability_margin;
		let stable = latest.number.saturating_sub(offset).into();
		let block = self.get_latest_block(stable, reference_timestamp).await?;
		Ok(block)
	}

	/// Finds a block by its `hash` and verifies that it is stable in reference to `reference_timestamp`
	/// and returns its info
	pub async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>> {
		let reference_timestamp = BlockDataSourceImpl::timestamp_to_db_type(reference_timestamp)?;
		self.get_stable_block_by_hash(hash, reference_timestamp).await
	}

	/// Finds a block by its `hash` and returns its info
	pub async fn get_block_by_hash(&self, hash: McBlockHash) -> Result<Option<MainchainBlock>> {
		let from_cache = if let Ok(cache) = self.stable_blocks_cache.lock() {
			cache.find_by_hash(hash.clone())
		} else {
			None
		};
		let block_opt = match from_cache {
			Some(block) => Some(block),
			None => Some(from_block_content(self.client.blocks_by_id(hash).await?)?),
		};
		Ok(block_opt)
	}
}

/// Configuration for [BlockDataSourceImpl]
#[derive(Debug, Clone, Deserialize)]
pub struct DolosBlockDataSourceConfig {
	/// Additional offset applied when selecting the latest stable Cardano block
	///
	/// This parameter should be 1 by default.
	pub block_stability_margin: u32,
}

impl DolosBlockDataSourceConfig {
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
		client: MiniBFClient,
	) -> std::result::Result<Self, Box<dyn Error + Send + Sync + 'static>> {
		Self::from_config(client, DolosBlockDataSourceConfig::from_env()?, &read_mc_epoch_config()?)
			.await
	}

	/// Creates a new instance of [BlockDataSourceImpl], using passed configuration.
	pub async fn from_config(
		client: MiniBFClient,
		DolosBlockDataSourceConfig { block_stability_margin }: DolosBlockDataSourceConfig,
		mc_epoch_config: &MainchainEpochConfig,
	) -> Result<BlockDataSourceImpl> {
		let genesis = client.genesis().await?;
		let active_slots_coeff = genesis.active_slots_coefficient;
		let security_parameter = genesis.security_param as u32;
		let k: f64 = security_parameter.into();
		let slot_duration: f64 = mc_epoch_config.slot_duration_millis.millis() as f64;
		let min_slot_boundary = (slot_duration * k / active_slots_coeff).round() as i64;
		let max_slot_boundary = 3 * min_slot_boundary;
		let cache_size = 100;
		Ok(BlockDataSourceImpl::new(
			client,
			security_parameter,
			TimeDelta::milliseconds(min_slot_boundary),
			TimeDelta::milliseconds(max_slot_boundary),
			mc_epoch_config.clone(),
			block_stability_margin,
			cache_size,
			BlocksCache::new_arc_mutex(),
		))
	}
	async fn get_latest_block(
		&self,
		max_block: McBlockNumber,
		reference_timestamp: NaiveDateTime,
	) -> Result<Option<MainchainBlock>> {
		let min_time_naive = self.min_block_allowed_time(reference_timestamp);
		let min_time = convert_naive_datetime(min_time_naive);
		let min_slot = self.date_time_to_slot(min_time_naive)?;
		let max_time_naive = self.max_allowed_block_time(reference_timestamp);
		let max_time = convert_naive_datetime(max_time_naive);
		let max_slot = self.date_time_to_slot(max_time_naive)?;

		let mut current_block_number = max_block;

		loop {
			let block = match self.client.blocks_by_id(current_block_number).await {
				Ok(b) => from_block_content(b)?,
				Err(_) => return Ok(None),
			};

			let is_time_match = block.timestamp >= min_time && block.timestamp <= max_time;
			let is_slot_match = block.slot >= min_slot && block.slot <= max_slot;

			if is_time_match && is_slot_match {
				return Ok(Some(block));
			}

			if block.timestamp < min_time || block.slot < min_slot || block.number.0 == 0 {
				return Ok(None);
			}

			current_block_number = block.number.saturating_sub(1u32);
		}
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
	fn is_block_time_valid(&self, block: &MainchainBlock, timestamp: NaiveDateTime) -> bool {
		convert_naive_datetime(self.min_block_allowed_time(timestamp)) <= block.timestamp
			&& block.timestamp <= convert_naive_datetime(self.max_allowed_block_time(timestamp))
	}

	async fn get_stable_block_by_hash(
		&self,
		hash: McBlockHash,
		reference_timestamp: NaiveDateTime,
	) -> Result<Option<MainchainBlock>> {
		if let Some(block) =
			self.get_stable_block_by_hash_from_cache(hash.clone(), reference_timestamp)
		{
			debug!("Block by hash: {hash} found in cache.");
			Ok(Some(From::from(block)))
		} else {
			debug!("Block by hash: {hash}, not found in cache, serving from Dolos.");
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
	) -> Option<MainchainBlock> {
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
	) -> Result<Option<MainchainBlock>> {
		let block = Some(from_block_content(self.client.blocks_by_id(hash).await?)?);
		let latest_block = Some(from_block_content(self.client.blocks_latest().await?)?);
		Ok(block
			.zip(latest_block)
			.filter(|(block, latest_block)| {
				block.number.saturating_add(self.security_parameter) <= latest_block.number
					&& self.is_block_time_valid(block, reference_timestamp)
			})
			.map(|(block, _)| block))
	}

	/// Caches stable blocks for lookup by hash.
	async fn fill_cache(&self, from_block: &MainchainBlock) -> Result<()> {
		let from_block_no = from_block.number;
		let size = u32::from(self.cache_size);
		let latest_block = from_block_content(self.client.blocks_latest().await?)?;
		let stable_block_num = latest_block.number.saturating_sub(self.security_parameter);

		let to_block_no = from_block_no.saturating_add(size).min(stable_block_num);
		let blocks = if from_block_no < to_block_no {
			let futures = (from_block_no.0..=to_block_no.0).map(|block_no| async move {
				self.client
					.blocks_by_id(McBlockNumber(block_no))
					.await
					.map_err(|e| e.into())
					.and_then(from_block_content)
			});
			futures::future::try_join_all(futures).await?.into_iter().collect()
		} else {
			vec![from_block.clone()]
		};

		if let Ok(mut cache) = self.stable_blocks_cache.lock() {
			cache.update(blocks);
			debug!("Cached blocks {} to {} for by hash lookups.", from_block_no.0, to_block_no.0);
		}
		Ok(())
	}

	fn date_time_to_slot(&self, dt: NaiveDateTime) -> Result<McSlotNumber> {
		let millis: u64 =
			dt.and_utc().timestamp_millis().try_into().map_err(|_| {
				DataSourceError::BadRequest(format!("Datetime out of range: {dt:?}"))
			})?;
		let ts = sidechain_domain::mainchain_epoch::Timestamp::from_unix_millis(millis);
		let slot = self
			.mainchain_epoch_config
			.timestamp_to_mainchain_slot_number(ts)
			.unwrap_or(self.mainchain_epoch_config.first_slot_number);
		Ok(McSlotNumber(slot))
	}

	fn timestamp_to_db_type(timestamp: Timestamp) -> Result<NaiveDateTime> {
		let millis: Option<i64> = timestamp.as_millis().try_into().ok();
		let dt = millis
			.and_then(DateTime::from_timestamp_millis)
			.ok_or(DataSourceError::BadRequest(format!("Timestamp out of range: {timestamp:?}")))?;
		Ok(NaiveDateTime::new(dt.date_naive(), dt.time()))
	}
}

fn convert_naive_datetime(d: NaiveDateTime) -> u64 {
	d.and_utc().timestamp().try_into().expect("i64 timestamp is valid u64")
}

/// Helper structure for caching stable blocks.
#[derive(new)]
pub(crate) struct BlocksCache {
	/// Continuous main chain blocks. All blocks should be stable. Used to query by hash.
	#[new(default)]
	from_last_by_hash: Vec<MainchainBlock>,
}

impl BlocksCache {
	fn find_by_hash(&self, hash: McBlockHash) -> Option<MainchainBlock> {
		self.from_last_by_hash.iter().find(|b| b.hash == hash).cloned()
	}

	pub fn update(&mut self, from_last_by_hash: Vec<MainchainBlock>) {
		self.from_last_by_hash = from_last_by_hash;
	}

	pub fn new_arc_mutex() -> Arc<Mutex<Self>> {
		Arc::new(Mutex::new(Self::new()))
	}
}
