use crate::Result;
use crate::block::BlockDataSourceImpl;
use async_trait::async_trait;
use sidechain_domain::*;
use sp_timestamp::Timestamp;
use std::sync::Arc;

pub struct McHashDataSourceImpl {
	/// [BlockDataSourceImpl] instance shared with other data sources for cache reuse.
	inner: Arc<BlockDataSourceImpl>,
}

impl McHashDataSourceImpl {
	pub fn new(inner: Arc<BlockDataSourceImpl>) -> Self {
		Self { inner }
	}
}

#[async_trait]
impl sidechain_mc_hash::McHashDataSource for McHashDataSourceImpl {
	async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>> {
		Ok(self
			.inner
			.get_latest_stable_block_for(Timestamp::new(reference_timestamp.as_millis()))
			.await?)
	}

	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>> {
		Ok(self
			.inner
			.get_stable_block_for(hash, Timestamp::new(reference_timestamp.as_millis()))
			.await?)
	}

	async fn get_block_by_hash(&self, hash: McBlockHash) -> Result<Option<MainchainBlock>> {
		Ok(self.inner.get_block_by_hash(hash).await?)
	}
}
