use std::marker::PhantomData;

use crate::{MainchainBlock, SidechainRpcDataSource};
use derive_new::new;
use jsonrpsee::core::async_trait;
use parity_scale_codec::Decode;

// The build.rs file of `substrate_test_runtime` is throwing an error. So a `Block` is being manually defined
pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

#[derive(Clone, PartialEq, Decode, Debug)]
pub(crate) struct TestSidechainParams(String);

#[allow(unused)]
pub(crate) fn mock_sidechain_params() -> TestSidechainParams {
	TestSidechainParams("test".to_string())
}

#[derive(new)]
pub struct SidechainRpcDataSourceMock<Err> {
	latest_block: MainchainBlock,
	_marker: PhantomData<Err>,
}

#[async_trait]
impl<Err: Send + Sync> SidechainRpcDataSource for SidechainRpcDataSourceMock<Err> {
	type Error = Err;

	async fn get_latest_block_info(&self) -> Result<MainchainBlock, Self::Error> {
		Ok(self.latest_block.clone())
	}
}
