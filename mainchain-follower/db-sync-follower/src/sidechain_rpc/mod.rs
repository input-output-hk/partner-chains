use crate::block::BlockDataSourceImpl;
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use crate::DataSourceError;
use async_trait::async_trait;
use main_chain_follower_api::BlockDataSource;
use pallet_sidechain_rpc::{MainchainBlock, SidechainRpcDataSource};
use std::sync::Arc;

pub struct SidechainRpcDataSourceImpl {
	inner: Arc<BlockDataSourceImpl>,
	metrics_opt: Option<McFollowerMetrics>,
}

impl SidechainRpcDataSourceImpl {
	pub fn new(inner: Arc<BlockDataSourceImpl>, metrics_opt: Option<McFollowerMetrics>) -> Self {
		Self { inner, metrics_opt }
	}
}

observed_async_trait!(
impl SidechainRpcDataSource for SidechainRpcDataSourceImpl {
	type Error = DataSourceError;

	async fn get_latest_block_info(&self) -> Result<MainchainBlock, Self::Error> {
		let block =
			<BlockDataSourceImpl as BlockDataSource>::get_latest_block_info(&self.inner)
				.await?;

		Ok(MainchainBlock { epoch: block.epoch, slot: block.slot })
	}
}
);
