//! Db-Sync data source used by Partner Chain Governed Map feature
use crate::DataSourceError::ExpectedDataNotFound;
use crate::Result;
use crate::block::BlockDataSourceImpl;
use crate::db_model::GovernedMapAction;
use crate::{metrics::McFollowerMetrics, observed_async_trait};
use db_sync_sqlx::{Asset, BlockNumber};
use itertools::Itertools;
use log::warn;
use partner_chains_plutus_data::governed_map::GovernedMapDatum;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sqlx::PgPool;
use std::cmp::{max, min};
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

/// Data source for the Governed Map feature of Partner Chains toolkit
///
/// See documentation of [sp_governed_map] for a description of the feature
pub struct GovernedMapDataSourceImpl {
	/// Postgres connection pool
	pub pool: PgPool,
	/// Prometheus metrics client
	pub metrics_opt: Option<McFollowerMetrics>,
}

impl GovernedMapDataSourceImpl {
	/// Creates a new instance of the data source
	pub async fn new(
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
	) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		crate::db_model::create_idx_tx_out_address(&pool).await?;
		Ok(Self { pool, metrics_opt })
	}
}

observed_async_trait!(
impl GovernedMapDataSource for GovernedMapDataSourceImpl {
	async fn get_state_at_block(
		&self,
		mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> std::result::Result<BTreeMap<String, ByteString>, Box<dyn std::error::Error + Send + Sync>>
	{
		Ok(get_mappings_entries(&self.pool, mc_block, scripts).await?.into())
	}

	async fn get_mapping_changes(
		&self,
		since_mc_block: Option<McBlockHash>,
		up_to_mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> std::result::Result<
		Vec<(String, Option<ByteString>)>,
		Box<dyn std::error::Error + Send + Sync>,
	> {
		let current_mappings = self.get_state_at_block(up_to_mc_block, scripts.clone()).await?;
		let Some(since_mc_block) = since_mc_block else {
			let changes =
				current_mappings.into_iter().map(|(key, value)| (key, Some(value))).collect();
			return Ok(changes);
		};
		let previous_mappings = self.get_state_at_block(since_mc_block, scripts.clone()).await?;
		let mut changes = vec![];
		for (key, value) in current_mappings.iter() {
			if previous_mappings.get(key) != Some(value) {
				changes.push((key.clone(), Some(value.clone())));
			}
		}
		for key in previous_mappings.keys() {
			if !current_mappings.contains_key(key) {
				changes.push((key.clone(), None));
			}
		}

		Ok(changes)
	}
}
);

async fn get_mappings_entries(
	pool: &PgPool,
	hash: McBlockHash,
	scripts: MainChainScriptsV1,
) -> Result<BTreeMap<String, ByteString>> {
	let Some(block) = crate::db_model::get_block_by_hash(pool, hash.clone()).await? else {
		return Err(ExpectedDataNotFound(format!("Block hash: {hash}")));
	};
	let entries = crate::db_model::get_datums_at_address_with_token(
		pool,
		&scripts.validator_address.into(),
		block.block_no,
		Asset::new(scripts.asset_policy_id),
	)
	.await?;

	let mut mappings = BTreeMap::new();
	for entry in entries {
		match GovernedMapDatum::try_from(entry.datum.0) {
			Ok(GovernedMapDatum { key, value }) => {
				mappings.insert(key, value);
			},
			Err(err) => warn!("Failed decoding map entry: {err}"),
		}
	}

	Ok(mappings)
}

/// Cached data source serving the Governed Map feature of Partner Chains toolkit
///
/// See documentation of [sp_governed_map] for a description of the feature
pub struct GovernedMapDataSourceCachedImpl {
	/// Postgres connection pool
	pub pool: PgPool,
	/// Prometheus metrics client
	pub metrics_opt: Option<McFollowerMetrics>,
	/// Internal data cache size
	cache_size: u16,
	/// Internal cache
	cache: Arc<Mutex<Cache>>,
	/// [BlockDataSourceImpl] instance shared with other data sources for cache reuse.
	blocks: Arc<BlockDataSourceImpl>,
}

impl GovernedMapDataSourceCachedImpl {
	/// Constructs a new Governed Map data source
	pub async fn new(
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
		cache_size: u16,
		blocks: Arc<BlockDataSourceImpl>,
	) -> std::result::Result<Self, Box<dyn std::error::Error + Send + Sync>> {
		crate::db_model::create_idx_tx_out_address(&pool).await?;
		let cache = Default::default();
		Ok(Self { pool, metrics_opt, cache_size, cache, blocks })
	}
}

observed_async_trait!(
impl GovernedMapDataSource for GovernedMapDataSourceCachedImpl {
	async fn get_state_at_block(
		&self,
		mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> std::result::Result<BTreeMap<String, ByteString>, Box<dyn std::error::Error + Send + Sync>>
	{
		Ok(get_mappings_entries(&self.pool, mc_block, scripts).await?.into())
	}

	async fn get_mapping_changes(
		&self,
		since_mc_block: Option<McBlockHash>,
		up_to_mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> std::result::Result<
		Vec<(String, Option<ByteString>)>,
		Box<dyn std::error::Error + Send + Sync>,
	> {
		if let Ok(mut cache) = self.cache.lock() {
			cache.set_main_chain_scripts(scripts.clone());
		};

		let since_block_number = match since_mc_block {
			Some(hash) => Some(
				crate::db_model::get_block_by_hash(&self.pool, hash.clone())
					.await?
					.ok_or_else(|| Box::new(ExpectedDataNotFound(format!("Block hash: {hash}"))))?
					.block_no,
			),
			None => None,
		};

		let Some(up_to_block) = self.blocks.get_block_by_hash(up_to_mc_block.clone()).await? else {
			return Err(Box::new(ExpectedDataNotFound(format!("Block hash: {up_to_mc_block}"))));
		};

		let up_to_block_number = BlockNumber(up_to_block.number.0);

		if let Some(cached_changes) =
			self.get_changes_from_cache(since_block_number, up_to_block_number).await?
		{
			return Ok(cached_changes);
		}

		let latest_block_timestamp = self.blocks.get_latest_block_info().await?.timestamp;
		let latest_stable_block =
			match self.blocks.get_latest_stable_block_for(latest_block_timestamp.into()).await? {
				Some(block) => BlockNumber(block.number.0),
				None => up_to_block_number,
			};
		let since_block_plus =
			BlockNumber(since_block_number.unwrap_or(BlockNumber(0)).0 + self.cache_size as u32);
		let max_search_block = min(latest_stable_block, max(up_to_block_number, since_block_plus));

		let changes = self
			.get_changes_in_range_to_cache(since_block_number, max_search_block, scripts)
			.await?;

		if let Ok(mut cache) = self.cache.lock() {
			cache.update(changes.clone());
		}

		Ok(filter_changes_in_range(changes, since_block_number, up_to_block_number))
	}
}
);

impl GovernedMapDataSourceCachedImpl {
	async fn get_changes_from_cache(
		&self,
		since_block: Option<BlockNumber>,
		up_to_block: BlockNumber,
	) -> Result<Option<Vec<(String, Option<ByteString>)>>> {
		if let Ok(cache) = self.cache.lock() {
			Ok(cache.get_changes_in_range(since_block, up_to_block))
		} else {
			Ok(None)
		}
	}

	async fn get_changes_in_range_to_cache(
		&self,
		since_block: Option<BlockNumber>,
		up_to_block: BlockNumber,
		scripts: MainChainScriptsV1,
	) -> Result<Vec<Change>> {
		let changes = crate::db_model::get_changes(
			&self.pool,
			&scripts.validator_address.into(),
			since_block,
			up_to_block,
			Asset::new(scripts.asset_policy_id),
		)
		.await?;

		let mut result = Vec::new();

		for change in changes {
			let GovernedMapDatum { key, value } = match GovernedMapDatum::try_from(change.datum.0) {
				Ok(datum) => datum,
				Err(err) => {
					warn!("Failed decoding map entry: {err}");
					continue;
				},
			};
			match change.action {
				GovernedMapAction::Spend => result.push(Change::new(change.block_no, key, None)),
				GovernedMapAction::Create => {
					result.push(Change::new(change.block_no, key, Some(value)))
				},
			};
		}
		Ok(result)
	}
}

#[derive(derive_new::new, Clone)]
struct Change {
	block_no: BlockNumber,
	key: String,
	value: Option<ByteString>,
}

#[derive(Default)]
pub(crate) struct Cache {
	highest_block_number: Option<BlockNumber>,
	lowest_block_number: Option<BlockNumber>,
	changes: Vec<Change>,
	address: Option<MainchainAddress>,
	policy_id: Option<PolicyId>,
}

fn filter_changes_in_range(
	changes: Vec<Change>,
	since_block: Option<BlockNumber>,
	up_to_block: BlockNumber,
) -> Vec<(String, Option<ByteString>)> {
	changes
		.into_iter()
		.filter(|change| {
			change.block_no.0 <= up_to_block.0
				&& since_block.map(|b| change.block_no.0 > b.0).unwrap_or(true)
		})
		.map(|change| (change.key, change.value))
		.collect()
}

impl Cache {
	fn get_changes_in_range(
		&self,
		since_block: Option<BlockNumber>,
		up_to_block: BlockNumber,
	) -> Option<Vec<(String, Option<ByteString>)>> {
		let Some(highest_block_number) = self.highest_block_number else {
			return None;
		};
		let Some(lowest_block_number) = self.lowest_block_number else {
			return None;
		};

		if highest_block_number.0 < up_to_block.0
			|| since_block.map(|b| b.0 < lowest_block_number.0).unwrap_or(false)
		{
			return None;
		}

		Some(filter_changes_in_range(self.changes.clone(), since_block, up_to_block))
	}

	fn update(&mut self, changes: Vec<Change>) {
		self.changes = changes;
		let (lowest_block_number, highest_block_number) = self
			.changes
			.iter()
			.minmax_by_key(|change| change.block_no.0)
			.into_option()
			.map(|(min, max)| (min.block_no, max.block_no))
			.unwrap_or((BlockNumber(0), BlockNumber(0)));
		self.lowest_block_number = Some(lowest_block_number);
		self.highest_block_number = Some(highest_block_number);
	}

	fn set_main_chain_scripts(&mut self, scripts: MainChainScriptsV1) {
		if self.address != Some(scripts.validator_address.clone())
			|| self.policy_id != Some(scripts.asset_policy_id.clone())
		{
			self.changes.clear();
			self.highest_block_number = None;
			self.lowest_block_number = None;
			self.address = Some(scripts.validator_address);
			self.policy_id = Some(scripts.asset_policy_id);
		}
	}
}
