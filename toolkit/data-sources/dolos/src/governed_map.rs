use crate::Result;
use async_trait::async_trait;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};

#[derive(Debug, Default)]
pub struct GovernedMapDataSourceImpl {}

#[async_trait]
impl GovernedMapDataSource for GovernedMapDataSourceImpl {
	async fn get_mapping_changes(
		&self,
		_since_mc_block: Option<McBlockHash>,
		_up_to_mc_block: McBlockHash,
		_scripts: MainChainScriptsV1,
	) -> Result<Vec<(String, Option<ByteString>)>> {
		Err("not implemented".into())
	}

	async fn get_state_at_block(
		&self,
		_mc_block: McBlockHash,
		_main_chain_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>> {
		Err("not implemented".into())
	}
}
