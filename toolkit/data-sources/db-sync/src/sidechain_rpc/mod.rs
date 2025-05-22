//! Db-Sync data source used by Partner Chain Json RPC
use crate::{block::BlockDataSourceImpl, metrics::McFollowerMetrics, observed_async_trait};
use pallet_sidechain_rpc::SidechainRpcDataSource;
use sidechain_domain::MainchainBlock;
use std::sync::Arc;

/// Db-Sync data source serving basic Cardano block data
pub struct SidechainRpcDataSourceImpl {
	/// [BlockDataSourceImpl] instance shared with other data sources for cache reuse.
	inner: Arc<BlockDataSourceImpl>,
	/// Prometheus metrics client
	metrics_opt: Option<McFollowerMetrics>,
}

impl SidechainRpcDataSourceImpl {
	/// Creates a Sidechain new data source by wrapping the given instance of [BlockDataSourceImpl]
	pub fn new(inner: Arc<BlockDataSourceImpl>, metrics_opt: Option<McFollowerMetrics>) -> Self {
		Self { inner, metrics_opt }
	}
}

observed_async_trait!(
impl SidechainRpcDataSource for SidechainRpcDataSourceImpl {
	async fn get_latest_block_info(
		&self,
	) -> Result<MainchainBlock, Box<dyn std::error::Error + Send + Sync>> {
		self.inner.get_latest_block_info().await
	}
}
);
