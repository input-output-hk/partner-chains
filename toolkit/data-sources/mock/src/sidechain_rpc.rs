use crate::Result;
use crate::block::BlockDataSourceMock;
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
	async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
		Ok(self.block_source.get_latest_block_info().await?)
	}
}
