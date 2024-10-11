use crate::block::BlockDataSourceImpl;
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use async_trait::async_trait;
use main_chain_follower_api::DataSourceError;
use main_chain_follower_api::{common::Timestamp, BlockDataSource};
use sidechain_domain::McBlockHash;
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
	) -> std::result::Result<Option<sidechain_mc_hash::MainchainBlock>, Self::Error> {
		Ok(<BlockDataSourceImpl as BlockDataSource>::get_latest_stable_block_for(
			&self.inner,
			Timestamp(reference_timestamp.as_millis()),
		)
		.await?
		.map(|block| sidechain_mc_hash::MainchainBlock {
			epoch: block.epoch,
			hash: block.hash,
			number: block.number,
			slot: block.slot,
			timestamp: block.timestamp,
		}))
	}

	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> std::result::Result<Option<sidechain_mc_hash::MainchainBlock>, Self::Error> {
		Ok(<BlockDataSourceImpl as BlockDataSource>::get_stable_block_for(
			&self.inner,
			hash,
			Timestamp(reference_timestamp.as_millis()),
		)
		.await?
		.map(|block| sidechain_mc_hash::MainchainBlock {
			epoch: block.epoch,
			hash: block.hash,
			number: block.number,
			slot: block.slot,
			timestamp: block.timestamp,
		}))
	}
}
);
