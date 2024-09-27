use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, NumberFor};
use std::collections::HashMap;

use crate::MainChainScripts;

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

pub type Hash = <Block as sp_runtime::traits::Block>::Hash;
pub type Header = <Block as sp_runtime::traits::Block>::Header;

#[derive(Clone)]
pub struct TestApi {
	pub headers: HashMap<<Block as BlockT>::Hash, <Block as BlockT>::Header>,
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

	}
}

impl HeaderBackend<Block> for TestApi {
	fn header(
		&self,
		id: <Block as BlockT>::Hash,
	) -> Result<Option<<Block as BlockT>::Header>, sp_blockchain::Error> {
		Ok(self.headers.get(&id).cloned())
	}

	fn info(&self) -> sp_blockchain::Info<Block> {
		unimplemented!()
	}

	fn status(
		&self,
		_id: <Block as BlockT>::Hash,
	) -> Result<sp_blockchain::BlockStatus, sp_blockchain::Error> {
		unimplemented!()
	}

	fn number(
		&self,
		_hash: <Block as BlockT>::Hash,
	) -> Result<Option<NumberFor<Block>>, sp_blockchain::Error> {
		unimplemented!()
	}

	fn hash(
		&self,
		_number: NumberFor<Block>,
	) -> Result<Option<<Block as BlockT>::Hash>, sp_blockchain::Error> {
		unimplemented!()
	}
}
