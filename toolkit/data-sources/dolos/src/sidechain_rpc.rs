use crate::{
	Result,
	client::{MiniBFClient, api::MiniBFApi, conversions::from_block_content},
};
use pallet_sidechain_rpc::SidechainRpcDataSource;
use sidechain_domain::MainchainBlock;

pub struct SidechainRpcDataSourceImpl {
	client: MiniBFClient,
}

impl SidechainRpcDataSourceImpl {
	pub fn new(client: MiniBFClient) -> Self {
		Self { client }
	}
}

#[async_trait::async_trait]
impl SidechainRpcDataSource for SidechainRpcDataSourceImpl {
	async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
		Ok(from_block_content(self.client.blocks_latest().await?)?)
	}
}
