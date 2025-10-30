use crate::{
	Result,
	client::{MiniBFClient, api::MiniBFApi, conversions::from_block_content},
};
use async_trait::async_trait;
use sidechain_domain::*;

pub struct McHashDataSourceImpl {
	client: MiniBFClient,
}

impl McHashDataSourceImpl {
	pub fn new(client: MiniBFClient) -> Self {
		Self { client }
	}
}

#[async_trait]
impl sidechain_mc_hash::McHashDataSource for McHashDataSourceImpl {
	async fn get_latest_stable_block_for(
		&self,
		_reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>> {
		Ok(Some(from_block_content(self.client.blocks_latest().await?)?))
	}

	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		_reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>> {
		Ok(Some(from_block_content(self.client.blocks_by_id(hash).await?)?))
	}

	async fn get_block_by_hash(&self, hash: McBlockHash) -> Result<Option<MainchainBlock>> {
		Ok(Some(from_block_content(self.client.blocks_by_id(hash).await?)?))
	}
}
