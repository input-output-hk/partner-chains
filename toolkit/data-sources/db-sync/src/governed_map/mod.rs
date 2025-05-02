use crate::DataSourceError::ExpectedDataNotFound;
use crate::Result;
use crate::block::BlockDataSourceImpl;
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
	cache_size: u16,
	#[new(default)]
	cache: Arc<Mutex<Cache>>,
	blocks: Arc<BlockDataSourceImpl>,
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
				self.blocks.get_block_by_hash(up_to_mc_block.clone()).await?
			else {
				return Err(Box::new(ExpectedDataNotFound(format!("Block hash: {up_to_mc_block}"))));
			};

			let up_to_block_number = BlockNumber(up_to_block.number.0);

			if let Some(cached_changes) = self.get_changes_from_cache(since_block_number, up_to_block_number).await? {
				return Ok(cached_changes);
			}

			let latest_stable_block = {
				match self.blocks.get_last_stable_block().await? {
					Some(block) => BlockNumber(block.number.0),
					None => up_to_block_number,
				}
			};
			let since_block_plus = BlockNumber(since_block_number.unwrap_or(BlockNumber(0)).0 + self.cache_size as u32);
			let max_search_block = min(latest_stable_block, max(up_to_block_number, since_block_plus));

			let changes = self.get_changes_in_range_to_cache(since_block_number, max_search_block, scripts).await?;

			if let Ok(mut cache) = self.cache.lock() {
				cache.update(changes.clone());
			}

			let filtered = filter_changes_in_range(changes, since_block_number, up_to_block_number);
			println!("filtered: {:?}", filtered);
			Ok(filtered)
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
	) -> Result<Vec<(BlockNumber, TxIndexInBlock, String, Option<ByteString>)>> {
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
			match GovernedMapDatum::try_from(change.datum.0) {
				Ok(GovernedMapDatum { key, value }) => match change.action.as_str() {
					"remove" => result.push((change.block_no, change.block_index, key, None)),
					"upsert" => {
						result.push((change.block_no, change.block_index, key, Some(value)))
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
	highest_block_number: Option<BlockNumber>,
	lowest_block_number: Option<BlockNumber>,
	changes: Vec<(BlockNumber, TxIndexInBlock, String, Option<ByteString>)>,
	address: Option<MainchainAddress>,
	policy_id: Option<PolicyId>,
}

fn filter_changes_in_range(
	changes: Vec<(BlockNumber, TxIndexInBlock, String, Option<ByteString>)>,
	since_block: Option<BlockNumber>,
	up_to_block: BlockNumber,
) -> Vec<(String, Option<ByteString>)> {
	changes
		.into_iter()
		.filter(|(block_number, _, _, _)| {
			block_number.0 <= up_to_block.0
				&& since_block.map(|b| block_number.0 > b.0).unwrap_or(true)
		})
		.map(|(_, _, key, value)| (key, value))
		.collect()
}

impl Cache {
	pub fn get_changes_in_range(
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

	pub fn update(
		&mut self,
		changes: Vec<(BlockNumber, TxIndexInBlock, String, Option<ByteString>)>,
	) {
		self.changes = changes;
		self.highest_block_number = Some(
			self.changes
				.iter()
				.max_by_key(|(block_number, _, _, _)| block_number.0)
				.map(|(block_number, _, _, _)| *block_number)
				.unwrap_or(BlockNumber(0)),
		);
		self.lowest_block_number = Some(
			self.changes
				.iter()
				.min_by_key(|(block_number, _, _, _)| block_number.0)
				.map(|(block_number, _, _, _)| *block_number)
				.unwrap_or(BlockNumber(0)),
		);
	}

	pub fn set_address(&mut self, address: MainchainAddress) {
		if self.address != Some(address.clone()) {
			self.changes.clear();
			self.highest_block_number = None;
			self.lowest_block_number = None;
			self.address = Some(address);
		}
	}

	pub fn set_asset(&mut self, policy_id: PolicyId) {
		if self.policy_id != Some(policy_id.clone()) {
			self.changes.clear();
			self.highest_block_number = None;
			self.lowest_block_number = None;
			self.policy_id = Some(policy_id);
		}
	}
}
