use crate::*;
use sidechain_domain::byte_string::ByteString;
use sp_api::ProvideRuntimeApi;
use std::collections::BTreeMap;

#[cfg(feature = "std")]
pub struct MockGovernedMapDataSource {
	pub current_mappings: Result<BTreeMap<String, ByteString>, String>,
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl GovernedMapDataSource for MockGovernedMapDataSource {
	async fn get_current_mappings(
		&self,
		_mc_block: McBlockHash,
		_main_chain_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self.current_mappings.clone()?)
	}
}

pub(crate) type BlockHash = sp_runtime::traits::BlakeTwo256;
pub(crate) type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, BlockHash>,
	sp_runtime::OpaqueExtrinsic,
>;

#[derive(Clone, Default)]
pub(crate) struct TestApiV1 {
	pub stored_mappings: BTreeMap<String, ByteString>,
}

impl ProvideRuntimeApi<Block> for TestApiV1 {
	type Api = Self;

	fn runtime_api(&self) -> sp_api::ApiRef<Self::Api> {
		(*self).clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl GovernedMapApi<Block> for TestApiV1 {
		fn get_stored_mappings() -> BTreeMap<String, ByteString> {
			self.stored_mappings.clone()
		}
		fn get_main_chain_scripts() -> Option<MainChainScriptsV1> {
			Some(Default::default())
		}
		fn get_pallet_version() -> u32 {
			1
		}
	}
}
