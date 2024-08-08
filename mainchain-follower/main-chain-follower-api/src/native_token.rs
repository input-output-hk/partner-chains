use crate::Result;
use async_trait::async_trait;
use sidechain_domain::*;

pub struct NativeTokenManagementReleaseEvent {
	pub token_amount: u64,
}

#[async_trait]
pub trait NativeTokenManagementDataSource {
	/// Retrieves all native token releases into the illiquid supply in the range (after_block, to_block]
	async fn get_token_release_events(
		&self,
		after_block: Option<McBlockHash>,
		to_block: McBlockHash,
		native_token_policy: PolicyId,
		illiquid_supply_address: MainchainAddress,
	) -> Result<Option<NativeTokenManagementReleaseEvent>>;
}
