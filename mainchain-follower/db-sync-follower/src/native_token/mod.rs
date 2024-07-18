use crate::db_model::Address;
use crate::metrics::McFollowerMetrics;
// use crate::observed_async_trait;
use crate::db_model::SlotNumber;
use async_trait::async_trait;
use main_chain_follower_api::native_token::NativeTokenManagementReleaseEvent;
use main_chain_follower_api::NativeTokenManagementDataSource;
use main_chain_follower_api::Result;
use sidechain_domain::*;
use sqlx::PgPool;

pub struct NativeTokenManagementDataSourceImpl {
	pub pool: PgPool,
	pub metrics_opt: Option<McFollowerMetrics>,
}

#[async_trait]
impl NativeTokenManagementDataSource for NativeTokenManagementDataSourceImpl {
	async fn get_token_release_events(
		&self,
		after_block: Option<McBlockHash>,
		to_block: McBlockHash,
		native_token_policy: PolicyId,
		illiquid_supply_address: MainchainAddress,
	) -> Result<Option<NativeTokenManagementReleaseEvent>> {
		if after_block == Some(to_block.clone()) {
			return Ok(None);
		}

		let after_slot = match after_block {
			None => SlotNumber(0),
			Some(after_block) => {
				crate::db_model::get_block_by_hash(&self.pool, after_block)
					.await?
					.expect("Parent MC hash is valid")
					.slot_no
			},
		};
		let to_slot = crate::db_model::get_block_by_hash(&self.pool, to_block)
			.await?
			.expect("curernt MC hash is valid")
			.slot_no;

		println!("Querying for token releases between slots {after_slot:?} - {to_slot:?}");
		let maybe_release = crate::db_model::get_native_token_releases(
			&self.pool,
			after_slot,
			to_slot,
			crate::db_model::PolicyId(native_token_policy.0.to_vec()),
			Address(illiquid_supply_address.to_string()),
		)
		.await?;

		Ok(maybe_release.0.map(|release| NativeTokenManagementReleaseEvent {
			token_amount: release.0.try_into().expect("i64 not a valid u64"),
		}))
	}
}
