use crate::Result;
use async_trait::async_trait;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};

#[derive(Debug, Default)]
pub struct GovernedMapDataSourceMock {
	mappings: BTreeMap<String, ByteString>,
}

impl GovernedMapDataSourceMock {
	pub fn new(mappings: BTreeMap<String, ByteString>) -> Self {
		Self { mappings }
	}
}

#[async_trait]
impl GovernedMapDataSource for GovernedMapDataSourceMock {
	async fn get_current_mappings(
		&self,
		_mc_block: McBlockHash,
		_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>> {
		Ok(self.mappings.clone())
	}
}
