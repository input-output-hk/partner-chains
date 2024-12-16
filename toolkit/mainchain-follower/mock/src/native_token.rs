use crate::Result;
use async_trait::async_trait;
use sidechain_domain::*;
use sp_native_token_management::{MainChainScripts, NativeTokenManagementDataSource};

pub struct NativeTokenDataSourceMock;

impl NativeTokenDataSourceMock {
	pub fn new() -> Self {
		Self
	}
}

impl Default for NativeTokenDataSourceMock {
	fn default() -> Self {
		Self::new()
	}
}

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
