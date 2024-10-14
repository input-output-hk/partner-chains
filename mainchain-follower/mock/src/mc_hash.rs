use crate::block::BlockDataSourceMock;
use async_trait::async_trait;
use main_chain_follower_api::{common::*, *};
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
	) -> std::result::Result<Option<MainchainBlock>, Self::Error> {
		Ok(self
			.block_source
			.get_latest_stable_block_for(Timestamp(reference_timestamp.as_millis()))
			.await?)
	}

	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> std::result::Result<Option<MainchainBlock>, Self::Error> {
		Ok(self
			.block_source
			.get_stable_block_for(hash, Timestamp(reference_timestamp.as_millis()))
			.await?)
	}
}
