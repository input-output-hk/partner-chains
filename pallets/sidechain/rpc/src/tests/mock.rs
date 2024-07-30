use parity_scale_codec::Decode;

// The build.rs file of `substrate_test_runtime` is throwing an error. So a `Block` is being manually defined
pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

#[derive(Clone, PartialEq, Decode, Debug)]
pub(crate) struct TestSidechainParams(String);

pub fn mock_sidechain_params() -> TestSidechainParams {
	TestSidechainParams("test".to_string())
}
