use crate::Result;
use async_trait::async_trait;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};

/// Mocked governed map data source that serves constant data
#[derive(Debug, Default)]
pub struct GovernedMapDataSourceMock {
	data: BTreeMap<String, ByteString>,
}

impl GovernedMapDataSourceMock {
	/// Creates new mock data source
	pub fn new(data: BTreeMap<String, ByteString>) -> Self {
		Self { data }
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
		Ok(self.data.iter().map(|(k, v)| (k.clone(), Some(v.clone()))).collect())
	}

	async fn get_state_at_block(
		&self,
		_mc_block: McBlockHash,
		_main_chain_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>> {
		Ok(self.data.clone())
	}
}
