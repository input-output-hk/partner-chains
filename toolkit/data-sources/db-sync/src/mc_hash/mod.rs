//! Db-Sync data source used by Partner Chain MC Reference feature
use crate::{block::BlockDataSourceImpl, metrics::McFollowerMetrics, observed_async_trait};
use sidechain_domain::{MainchainBlock, McBlockHash};
use sidechain_mc_hash::McHashDataSource;
use sp_timestamp::Timestamp;
use std::sync::Arc;

/// Db-Sync data source used by the Main Chain Reference feature of Partner Chain toolkit.
///
/// See [sidechain_mc_hash] a detailed explanation of its purpose.
pub struct McHashDataSourceImpl {
	/// [BlockDataSourceImpl] instance shared with other data sources for cache reuse.
	inner: Arc<BlockDataSourceImpl>,
	/// Prometheus metrics client
	metrics_opt: Option<McFollowerMetrics>,
}

impl McHashDataSourceImpl {
	/// Creates a new MC Hash data source by wrapping an instance of [BlockDataSourceImpl]
	pub fn new(inner: Arc<BlockDataSourceImpl>, metrics_opt: Option<McFollowerMetrics>) -> Self {
		Self { inner, metrics_opt }
	}
}

observed_async_trait!(
impl McHashDataSource for McHashDataSourceImpl {
	async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> std::result::Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self
			.inner
			.get_latest_stable_block_for(Timestamp::new(reference_timestamp.as_millis()))
			.await?)
	}

	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> std::result::Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self
			.inner
			.get_stable_block_for(hash, Timestamp::new(reference_timestamp.as_millis()))
			.await?)
	}

	async fn get_block_by_hash(
		&self,
		hash: McBlockHash,
	) -> std::result::Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self.inner.get_block_by_hash(hash).await?)
	}
}
);
