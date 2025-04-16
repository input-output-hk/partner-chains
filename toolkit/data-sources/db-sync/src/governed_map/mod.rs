use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use crate::DataSourceError::ExpectedDataNotFound;
use crate::Result;
use derive_new::new;
use log::warn;
use partner_chains_plutus_data::governed_map::GovernedMapDatum;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sqlx::PgPool;

#[cfg(test)]
pub mod tests;

#[derive(new)]
pub struct GovernedMapDataSourceImpl {
	pub pool: PgPool,
	pub metrics_opt: Option<McFollowerMetrics>,
}

observed_async_trait!(
impl GovernedMapDataSource for GovernedMapDataSourceImpl {
	async fn get_current_mappings(
		&self,
		mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> std::result::Result<BTreeMap<String, ByteString>, Box<dyn std::error::Error + Send + Sync>>
	{
		let mut governed_map = BTreeMap::new();
		let entries = self.get_current_mapping_entries(mc_block, scripts).await?;
		for entry in entries {
			match GovernedMapDatum::try_from(entry.datum.0) {
				Ok(GovernedMapDatum{key, value}) => {
					governed_map.insert(key, value);
				},
				Err(err) => warn!("Failed decoding map entry: {err}"),
			}
		}
		Ok(governed_map)
	}
}
);

impl GovernedMapDataSourceImpl {
	async fn get_current_mapping_entries(
		&self,
		hash: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> Result<Vec<crate::db_model::DatumOutput>> {
		let Some(block) = crate::db_model::get_block_by_hash(&self.pool, hash.clone()).await?
		else {
			return Err(ExpectedDataNotFound(format!("Block hash: {hash}")));
		};
		Ok(crate::db_model::get_datums_at_address_with_token(
			&self.pool,
			&scripts.validator_address.into(),
			block.block_no,
			scripts.asset.into(),
		)
		.await?)
	}
}
