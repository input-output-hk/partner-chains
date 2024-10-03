use crate::db_model::{Address, NativeTokenAmount, SlotNumber};
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use async_trait::async_trait;
use derive_new::new;
use main_chain_follower_api::{DataSourceError, NativeTokenManagementDataSource, Result};
use sidechain_domain::*;
use sqlx::PgPool;

#[cfg(test)]
mod tests;

#[derive(new)]
pub struct NativeTokenManagementDataSourceImpl {
	pub pool: PgPool,
	pub metrics_opt: Option<McFollowerMetrics>,
}

observed_async_trait!(
impl NativeTokenManagementDataSource for NativeTokenManagementDataSourceImpl {
	async fn get_total_native_token_transfer(
		&self,
		after_block: Option<McBlockHash>,
		to_block: McBlockHash,
		native_token_policy_id: PolicyId,
		native_token_asset_name: AssetName,
		illiquid_supply_address: MainchainAddress,
	) -> Result<sidechain_domain::NativeTokenAmount> {
		if after_block == Some(to_block.clone()) {
			return Ok(NativeTokenAmount(0).into());
		}

		let (after_slot , to_slot) = futures::try_join!(
			get_after_slot(after_block, &self.pool),
			get_to_slot(to_block, &self.pool)
		)?;

		let total_transfer = crate::db_model::get_total_native_tokens_transfered(
			&self.pool,
			after_slot,
			to_slot,
			crate::db_model::Asset {
				policy_id: native_token_policy_id.into(),
				asset_name: native_token_asset_name.into(),
			},
			Address(illiquid_supply_address.to_string()),
		)
		.await?;

		Ok(total_transfer.unwrap_or(NativeTokenAmount(0)).into())
	}
}
);

async fn get_after_slot(after_block: Option<McBlockHash>, pool: &PgPool) -> Result<SlotNumber> {
	match after_block {
		None => Ok(SlotNumber(0)),
		Some(after_block) => Ok(crate::db_model::get_block_by_hash(pool, after_block.clone())
			.await?
			.ok_or(DataSourceError::ExpectedDataNotFound(format!(
				"Lower bound block {after_block} not found when querying for native token transfers"
			)))?
			.slot_no),
	}
}

async fn get_to_slot(to_block: McBlockHash, pool: &PgPool) -> Result<SlotNumber> {
	Ok(crate::db_model::get_block_by_hash(pool, to_block.clone())
		.await?
		.ok_or(DataSourceError::ExpectedDataNotFound(format!(
			"Upper bound block {to_block} not found when querying for native token transfers"
		)))?
		.slot_no)
}
