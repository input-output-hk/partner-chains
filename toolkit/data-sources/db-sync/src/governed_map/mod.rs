use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use crate::DataSourceError::ExpectedDataNotFound;
use crate::Result;
use cardano_serialization_lib::PlutusData;
use derive_new::new;
use log::warn;
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
			match plutus_data_to_key_value(&entry.datum.0) {
				Ok((key, val)) => {governed_map.insert(key, val);},
				Err(err) => warn!("Failed decoding map entry: {err}"),
			}
		}
		Ok(governed_map)
	}
}
);

fn plutus_data_to_key_value(
	plutus_data: &PlutusData,
) -> std::result::Result<(String, ByteString), String> {
	Ok(plutus_data
		.as_list()
		.filter(|datum| datum.len() == 2)
		.ok_or("Expected 2 element list datum")
		.and_then(|items| {
			let key: String =
				String::from_utf8(items.get(0).as_bytes().ok_or("Key is not plutus bytes")?)
					.map_err(|_| "Key is not valid UTF-8")?;
			let val: ByteString =
				items.get(1).as_bytes().ok_or("Value is not plutus bytes")?.into();
			Ok((key, val))
		})?)
}

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

		let asset = crate::db_model::Asset {
			policy_id: scripts.asset.policy_id.into(),
			asset_name: scripts.asset.asset_name.into(),
		};
		Ok(crate::db_model::get_datums_at_address_with_token(
			&self.pool,
			&scripts.validator_address.into(),
			block.block_no,
			asset,
		)
		.await?)
	}
}
