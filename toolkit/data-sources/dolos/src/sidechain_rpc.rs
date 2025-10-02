use crate::Result;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use sidechain_domain::MainchainBlock;

pub struct SidechainRpcDataSourceImpl {}

impl SidechainRpcDataSourceImpl {
	pub fn new() -> Self {
		Self {}
	}
}

#[async_trait::async_trait]
impl SidechainRpcDataSource for SidechainRpcDataSourceImpl {
	async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
		Err("not implemented".into())
	}
}
