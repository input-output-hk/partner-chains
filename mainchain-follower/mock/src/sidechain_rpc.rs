use crate::block::BlockDataSourceMock;
use main_chain_follower_api::{BlockDataSource, DataSourceError};
use pallet_sidechain_rpc::{MainchainBlock, SidechainRpcDataSource};
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
		let block =
			<BlockDataSourceMock as BlockDataSource>::get_latest_block_info(&self.block_source)
				.await?;

		Ok(MainchainBlock { epoch: block.epoch, slot: block.slot })
	}
}
