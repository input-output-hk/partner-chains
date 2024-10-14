use crate::{
	block::BlockDataSourceImpl, metrics::McFollowerMetrics, observed_async_trait, DataSourceError,
};
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
	type Error = DataSourceError;

	async fn get_latest_block_info(&self) -> Result<MainchainBlock, Self::Error> {
		let block = self.inner.get_latest_block_info().await?;

		Ok(block)
	}
}
);
