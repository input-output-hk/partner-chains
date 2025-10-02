use crate::Result;
use async_trait::async_trait;
use sidechain_domain::*;

pub struct McHashDataSourceImpl {}

impl McHashDataSourceImpl {
	pub fn new() -> Self {
		Self {}
	}
}

#[async_trait]
impl sidechain_mc_hash::McHashDataSource for McHashDataSourceImpl {
	async fn get_latest_stable_block_for(
		&self,
		_reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>> {
		Err("not implemented".into())
	}

	async fn get_stable_block_for(
		&self,
		_hash: McBlockHash,
		_reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>> {
		Err("not implemented".into())
	}

	async fn get_block_by_hash(&self, _hash: McBlockHash) -> Result<Option<MainchainBlock>> {
		Err("not implemented".into())
	}
}
