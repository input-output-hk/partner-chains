use crate::block::BlockDataSourceMock;
use async_trait::async_trait;
pub use main_chain_follower_api::block::*;
use main_chain_follower_api::common::*;
use main_chain_follower_api::*;
use sidechain_domain::*;
use std::sync::Arc;

pub struct McHashDataSourceMock {
	block_source: Arc<BlockDataSourceMock>,
}

impl McHashDataSourceMock {
	pub fn new(inner: Arc<BlockDataSourceMock>) -> Self {
		Self { block_source: inner }
	}
}

#[async_trait]
impl sidechain_mc_hash::McHashDataSource for McHashDataSourceMock {
	type Error = DataSourceError;

	async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> std::result::Result<Option<sidechain_mc_hash::MainchainBlock>, Self::Error> {
		Ok(<BlockDataSourceMock as BlockDataSource>::get_latest_stable_block_for(
			&self.block_source,
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
		Ok(<BlockDataSourceMock as BlockDataSource>::get_stable_block_for(
			&self.block_source,
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
