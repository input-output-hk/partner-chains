use crate::Result;
use async_trait::async_trait;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};

#[derive(Debug, Default)]
pub struct GovernedMapDataSourceMock {
	changes: Vec<(String, Option<ByteString>)>,
}

impl GovernedMapDataSourceMock {
	pub fn new(changes: Vec<(String, Option<ByteString>)>) -> Self {
		Self { changes }
	}
}

#[async_trait]
impl GovernedMapDataSource for GovernedMapDataSourceMock {
	async fn get_mapping_changes(
		&self,
		_since_mc_block: Option<McBlockHash>,
		_up_to_mc_block: McBlockHash,
		_scripts: MainChainScriptsV1,
	) -> Result<Vec<(String, Option<ByteString>)>> {
		Ok(self.changes.clone())
	}
}
