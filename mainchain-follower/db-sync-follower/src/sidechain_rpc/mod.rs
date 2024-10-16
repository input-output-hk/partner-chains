use crate::{block::BlockDataSourceImpl, metrics::McFollowerMetrics, observed_async_trait};
use async_trait::async_trait;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use sidechain_domain::MainchainBlock;
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
	async fn get_latest_block_info(&self) -> Result<MainchainBlock, Box<dyn std::error::Error>> {
		Ok(self.inner.get_latest_block_info().await?)
	}
}
);
