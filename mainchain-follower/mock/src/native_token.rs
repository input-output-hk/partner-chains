use async_trait::async_trait;
use main_chain_follower_api::native_token::*;
use main_chain_follower_api::*;
use sidechain_domain::*;

pub struct NativeTokenDataSourceMock;

impl NativeTokenDataSourceMock {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait]
impl NativeTokenManagementDataSource for NativeTokenDataSourceMock {
	async fn get_total_native_token_transfer(
		&self,
		_after_block: Option<McBlockHash>,
		_to_block: McBlockHash,
		_native_token_policy_id: PolicyId,
		_native_token_asset_name: AssetName,
		_illiquid_supply_address: MainchainAddress,
	) -> Result<NativeTokenAmount> {
		Ok(NativeTokenAmount(1000))
	}
}
