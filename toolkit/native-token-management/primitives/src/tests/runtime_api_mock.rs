use crate::MainChainScripts;

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

pub type Hash = <Block as sp_runtime::traits::Block>::Hash;

#[derive(Clone)]
pub struct TestApi {
	pub main_chain_scripts: Option<MainChainScripts>,
}

impl sp_api::ProvideRuntimeApi<Block> for TestApi {
	type Api = TestApi;

	fn runtime_api(&self) -> sp_api::ApiRef<Self::Api> {
		self.clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl crate::NativeTokenManagementApi<Block> for TestApi {
		fn get_main_chain_scripts() -> Option<MainChainScripts> {
			self.main_chain_scripts.clone()
		}

		fn initialized() -> bool {
			true
		}

	}
}
