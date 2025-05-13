use crate::Result;
use async_trait::async_trait;
use sidechain_domain::*;
use sp_native_token_management::{MainChainScripts, NativeTokenManagementDataSource};

#[derive(Default)]
pub struct NativeTokenDataSourceMock;

#[async_trait]
impl NativeTokenManagementDataSource for NativeTokenDataSourceMock {
	async fn get_total_native_token_transfer(
		&self,
		_after_block: Option<McBlockHash>,
		_to_block: McBlockHash,
		_scripts: MainChainScripts,
	) -> Result<NativeTokenAmount> {
		Ok(NativeTokenAmount(1000))
	}
}
