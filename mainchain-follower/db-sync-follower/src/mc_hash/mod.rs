use crate::{block::BlockDataSourceImpl, metrics::McFollowerMetrics, observed_async_trait};
use async_trait::async_trait;
use main_chain_follower_api::{common::Timestamp, DataSourceError};
use sidechain_domain::{MainchainBlock, McBlockHash};
use sidechain_mc_hash::McHashDataSource;
use std::sync::Arc;

pub struct McHashDataSourceImpl {
	inner: Arc<BlockDataSourceImpl>,
	metrics_opt: Option<McFollowerMetrics>,
}

impl McHashDataSourceImpl {
	pub fn new(inner: Arc<BlockDataSourceImpl>, metrics_opt: Option<McFollowerMetrics>) -> Self {
		Self { inner, metrics_opt }
	}
}

observed_async_trait!(
impl McHashDataSource for McHashDataSourceImpl {
	type Error = DataSourceError;

	async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> std::result::Result<Option<MainchainBlock>, Self::Error> {
		Ok(self
			.inner
			.get_latest_stable_block_for(Timestamp(reference_timestamp.as_millis()))
			.await?)
	}

	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> std::result::Result<Option<MainchainBlock>, Self::Error> {
		Ok(self
			.inner
			.get_stable_block_for(hash, Timestamp(reference_timestamp.as_millis()))
			.await?)
	}
}
);
