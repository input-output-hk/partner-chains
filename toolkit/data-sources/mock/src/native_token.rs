use crate::Result;
use async_trait::async_trait;
use sidechain_domain::*;
use sp_native_token_management::{MainChainScripts, NativeTokenManagementDataSource};

/// Mock native token data source that serves constant data
#[derive(Default)]
pub struct NativeTokenDataSourceMock;

impl NativeTokenDataSourceMock {
	/// Creates new mocked native token data source
	pub fn new() -> Self {
		Self
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
