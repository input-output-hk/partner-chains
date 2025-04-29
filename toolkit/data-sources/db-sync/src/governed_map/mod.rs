use crate::DataSourceError::{self, ExpectedDataNotFound};
use crate::Result;
use crate::db_model::Block;
use crate::{metrics::McFollowerMetrics, observed_async_trait};
use db_sync_sqlx::Asset;
use db_sync_sqlx::{BlockNumber, TxIndexInBlock};
use derive_new::new;
use log::warn;
use partner_chains_plutus_data::governed_map::GovernedMapDatum;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sqlx::PgPool;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(test)]
pub mod tests;

#[derive(new)]
pub struct GovernedMapDataSourceImpl {
	pub pool: PgPool,
	pub metrics_opt: Option<McFollowerMetrics>,
}

observed_async_trait!(
impl GovernedMapDataSource for GovernedMapDataSourceImpl {
	async fn get_mapping_changes(
		&self,
		since_mc_block: Option<McBlockHash>,
		up_to_mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> std::result::Result<
		Vec<(String, Option<ByteString>)>,
		Box<dyn std::error::Error + Send + Sync>,
	> {
		let current_mappings =
			self.get_current_mapping_entries(up_to_mc_block, scripts.clone()).await?;
		let Some(since_mc_block) = since_mc_block else {
			let changes =
				current_mappings.into_iter().map(|(key, value)| (key, Some(value))).collect();
			return Ok(changes);
		};
		let previous_mappings =
			self.get_current_mapping_entries(since_mc_block, scripts.clone()).await?;
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

impl GovernedMapDataSourceImpl {
	async fn get_current_mapping_entries(
		&self,
		hash: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> Result<HashMap<String, ByteString>> {
		let Some(block) = crate::db_model::get_block_by_hash(&self.pool, hash.clone()).await?
		else {
			return Err(ExpectedDataNotFound(format!("Block hash: {hash}")));
		};
		let entries = crate::db_model::get_datums_at_address_with_token(
			&self.pool,
			&scripts.validator_address.into(),
			block.block_no,
			Asset::new(scripts.asset_policy_id),
		)
		.await?;

		let mut mappings = HashMap::new();
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
}

#[derive(new)]
pub struct GovernedMapDataSourceCachedImpl {
	pub pool: PgPool,
	pub metrics_opt: Option<McFollowerMetrics>,
	security_parameter: u32,
	cache_size: u16,
	#[new(default)]
	cache: Arc<Mutex<Cache>>,
}

observed_async_trait!(
	impl GovernedMapDataSource for GovernedMapDataSourceCachedImpl {
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
				cache.set_address(scripts.validator_address.clone().into());
				cache.set_asset(scripts.asset_policy_id.clone());
			};

			let since_block_number = match since_mc_block {
				Some(hash) =>
					Some(crate::db_model::get_block_by_hash(&self.pool, hash.clone())
						.await?
						.ok_or_else(|| Box::new(ExpectedDataNotFound(format!("Block hash: {hash}"))) as Box<dyn std::error::Error + Send + Sync>)?
						.block_no),
				None => None,
			};

			let Some(up_to_block) =
				crate::db_model::get_block_by_hash(&self.pool, up_to_mc_block.clone()).await?
			else {
				return Err(Box::new(ExpectedDataNotFound(format!("Block hash: {up_to_mc_block}"))));
			};

			if let Some(cached_changes) = self.get_changes_from_cache(since_block_number, up_to_block.block_no).await? {
				return Ok(cached_changes);
			}

			let latest_block = get_latest_block(&self.pool).await?;
			let latest_stable_block = BlockNumber(latest_block.block_no.0.saturating_sub(self.security_parameter));
			let since_block_plus = BlockNumber(since_block_number.unwrap_or(BlockNumber(0)).0 + self.cache_size as u32);
			let max_search_block = min(latest_stable_block, max(up_to_block.block_no, since_block_plus));

			let changes = self.get_changes_in_range_to_cache(since_block_number, max_search_block, scripts).await?;

			if let Ok(mut cache) = self.cache.lock() {
				cache.add_changes(changes.clone());
			}

			Ok(deduplicate_changes(filter_changes_in_range(changes, since_block_number, up_to_block.block_no)))
		}
	}
);

impl GovernedMapDataSourceCachedImpl {
	pub fn new_from_env(
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
	) -> std::result::Result<Self, &'static str> {
		let security_parameter: u32 = std::env::var("CARDANO_SECURITY_PARAMETER")
			.ok()
			.and_then(|s| s.parse().ok())
			.ok_or("Couldn't read env variable CARDANO_SECURITY_PARAMETER as u32")?;
		Ok(Self {
			pool,
			metrics_opt,
			security_parameter,
			cache_size: 1000,
			cache: Default::default(),
		})
	}

	async fn get_changes_from_cache(
		&self,
		since_block: Option<BlockNumber>,
		up_to_block: BlockNumber,
	) -> Result<Option<Vec<(String, Option<ByteString>)>>> {
		if let Ok(cache) = self.cache.lock() {
			if let Some(changes) = cache.get_changes_in_range(since_block, up_to_block) {
				return Ok(Some(deduplicate_changes(changes)));
			}
		};
		return Ok(None);
	}

	async fn get_changes_in_range_to_cache(
		&self,
		since_block: Option<BlockNumber>,
		up_to_block: BlockNumber,
		scripts: MainChainScriptsV1,
	) -> Result<Vec<(String, Option<ByteString>, BlockNumber, TxIndexInBlock)>> {
		let latest_block_number = {
			let cache = self.cache.lock().expect("Failed to lock cache");

			cache.newest_block_number.or(since_block)
		};
		let changes = crate::db_model::get_changes(
			&self.pool,
			&scripts.validator_address.into(),
			latest_block_number,
			up_to_block,
			Asset::new(scripts.asset_policy_id),
		)
		.await?;

		let mut result = Vec::new();

		for change in changes {
			match GovernedMapDatum::try_from(change.datum.0) {
				Ok(GovernedMapDatum { key, value }) => match change.action.as_str() {
					"remove" => result.push((key, None, change.block_no, change.block_index)),
					"upsert" => {
						result.push((key, Some(value), change.block_no, change.block_index))
					},
					_ => warn!("Unknown action: {}", change.action),
				},
				Err(err) => warn!("Failed decoding map entry: {err}"),
			}
		}
		Ok(result)
	}
}

#[derive(Default)]
pub(crate) struct Cache {
	newest_block_number: Option<BlockNumber>,
	changes: Vec<(String, Option<ByteString>, BlockNumber, TxIndexInBlock)>,
	address: Option<MainchainAddress>,
	policy_id: Option<PolicyId>,
}

fn deduplicate_changes(
	mut changes: Vec<(String, Option<ByteString>, BlockNumber, TxIndexInBlock)>,
) -> Vec<(String, Option<ByteString>)> {
	let mut result = HashMap::new();

	changes.sort_by_key(|(_, _, block_number, block_index)| (*block_number, *block_index));

	for (key, value, _, _) in changes {
		if result.contains_key(&key) {
			result.remove(&key);
		}
		result.insert(key, value);
	}

	result.into_iter().collect()
}

fn filter_changes_in_range(
	changes: Vec<(String, Option<ByteString>, BlockNumber, TxIndexInBlock)>,
	since_block: Option<BlockNumber>,
	up_to_block: BlockNumber,
) -> Vec<(String, Option<ByteString>, BlockNumber, TxIndexInBlock)> {
	changes
		.iter()
		.filter(|(_, _, block_number, _)| {
			block_number.0 <= up_to_block.0
				&& since_block.map(|b| block_number.0 > b.0).unwrap_or(true)
		})
		.map(|x| x.clone())
		.collect()
}

async fn get_latest_block(pool: &PgPool) -> Result<Block> {
	crate::db_model::get_latest_block_info(pool).await?.ok_or(
		DataSourceError::ExpectedDataNotFound(
			"The latest block not found when querying for native token transfers".to_string(),
		),
	)
}

impl Cache {
	pub fn get_changes_in_range(
		&self,
		since_block: Option<BlockNumber>,
		up_to_block: BlockNumber,
	) -> Option<Vec<(String, Option<ByteString>, BlockNumber, TxIndexInBlock)>> {
		let Some(newest_block_number) = self.newest_block_number else {
			return None;
		};

		if newest_block_number.0 < up_to_block.0 {
			return None;
		}

		Some(filter_changes_in_range(self.changes.clone(), since_block, up_to_block))
	}

	pub fn add_changes(
		&mut self,
		changes: Vec<(String, Option<ByteString>, BlockNumber, TxIndexInBlock)>,
	) {
		self.changes.extend(changes);
		self.newest_block_number = Some(
			self.changes
				.iter()
				.max_by_key(|(_, _, block_number, _)| block_number.0)
				.map(|(_, _, block_number, _)| *block_number)
				.unwrap_or(BlockNumber(0)),
		);
	}

	pub fn set_address(&mut self, address: MainchainAddress) {
		if self.address != Some(address.clone()) {
			self.changes.clear();
			self.newest_block_number = None;
			self.address = Some(address);
		}
	}

	pub fn set_asset(&mut self, policy_id: PolicyId) {
		if self.policy_id != Some(policy_id.clone()) {
			self.changes.clear();
			self.newest_block_number = None;
			self.policy_id = Some(policy_id);
		}
	}
}
