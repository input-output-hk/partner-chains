use crate::block::BlockDataSourceMock;
use main_chain_follower_api::DataSourceError;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use sidechain_domain::MainchainBlock;
use std::sync::Arc;

pub struct SidechainRpcDataSourceMock {
	block_source: Arc<BlockDataSourceMock>,
}

impl SidechainRpcDataSourceMock {
	pub fn new(inner: Arc<BlockDataSourceMock>) -> Self {
		Self { block_source: inner }
	}
}

#[async_trait::async_trait]
impl SidechainRpcDataSource for SidechainRpcDataSourceMock {
	type Error = DataSourceError;

	async fn get_latest_block_info(&self) -> Result<MainchainBlock, Self::Error> {
		let block = self.block_source.get_latest_block_info().await?;

		Ok(block)
	}
}
