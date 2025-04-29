use crate::DataSourceError::ExpectedDataNotFound;
use crate::Result;
use crate::{metrics::McFollowerMetrics, observed_async_trait};
use db_sync_sqlx::Asset;
use derive_new::new;
use log::warn;
use partner_chains_plutus_data::governed_map::GovernedMapDatum;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sqlx::PgPool;
use std::collections::HashMap;

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
