use crate::Result;
use async_trait::async_trait;
use sidechain_domain::*;

#[async_trait]
pub trait NativeTokenManagementDataSource {
	/// Retrieves total of native token transfers into the illiquid supply in the range (after_block, to_block]
	async fn get_total_native_token_transfer(
		&self,
		after_block: Option<McBlockHash>,
		to_block: McBlockHash,
		native_token_policy_id: PolicyId,
		native_token_asset_name: AssetName,
		illiquid_supply_address: MainchainAddress,
	) -> Result<NativeTokenAmount>;
}
