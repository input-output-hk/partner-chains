use crate::*;
use sidechain_domain::byte_string::ByteString;
use sp_api::ProvideRuntimeApi;

#[cfg(feature = "std")]
pub struct MockGovernedMapDataSource {
	pub changes: Vec<(String, Option<ByteString>)>,
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl GovernedMapDataSource for MockGovernedMapDataSource {
	async fn get_mapping_changes(
		&self,
		_since_mc_block: Option<McBlockHash>,
		_up_to_mc_block: McBlockHash,
		_main_chain_scripts: MainChainScriptsV1,
	) -> Result<Vec<(String, Option<ByteString>)>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self.changes.clone())
	}
}

pub(crate) type BlockHash = sp_runtime::traits::BlakeTwo256;
pub(crate) type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, BlockHash>,
	sp_runtime::OpaqueExtrinsic,
>;

#[derive(Clone, Default)]
pub(crate) struct TestApiV1;

impl ProvideRuntimeApi<Block> for TestApiV1 {
	type Api = Self;

	fn runtime_api(&self) -> sp_api::ApiRef<Self::Api> {
		(*self).clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl GovernedMapIDPApi<Block> for TestApiV1 {
		fn get_main_chain_scripts() -> Option<MainChainScriptsV1> {
			Some(Default::default())
		}
		fn get_pallet_version() -> u32 {
			1
		}
	}
}
