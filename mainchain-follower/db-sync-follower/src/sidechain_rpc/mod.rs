use crate::block::BlockDataSourceImpl;
use crate::DataSourceError;
use main_chain_follower_api::BlockDataSource;
use pallet_sidechain_rpc::{MainchainBlock, SidechainRpcDataSource};
use std::sync::Arc;

pub struct SidechainRpcDataSourceImpl {
	block_source: Arc<BlockDataSourceImpl>,
}

impl SidechainRpcDataSourceImpl {
	pub fn new(inner: Arc<BlockDataSourceImpl>) -> Self {
		Self { block_source: inner }
	}
}

#[async_trait::async_trait]
impl SidechainRpcDataSource for SidechainRpcDataSourceImpl {
	type Error = DataSourceError;

	async fn get_latest_block_info(&self) -> Result<MainchainBlock, Self::Error> {
		let block =
			<BlockDataSourceImpl as BlockDataSource>::get_latest_block_info(&self.block_source)
				.await?;

		Ok(MainchainBlock { epoch: block.epoch, slot: block.slot })
	}
}
