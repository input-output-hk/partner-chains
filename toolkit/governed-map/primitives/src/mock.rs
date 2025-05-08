use crate::*;
use sidechain_domain::byte_string::ByteString;
use sp_api::ProvideRuntimeApi;

#[cfg(feature = "std")]
#[derive(Debug, Default)]
pub struct MockGovernedMapDataSource {
	pub changes: Vec<(String, Option<ByteString>)>,
	pub data: BTreeMap<String, ByteString>,
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

	async fn get_state_at_block(
		&self,
		_mc_block: McBlockHash,
		_main_chain_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self.data.clone())
	}
}

pub(crate) type BlockHash = sp_runtime::traits::BlakeTwo256;
pub(crate) type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, BlockHash>,
	sp_runtime::OpaqueExtrinsic,
>;

#[derive(Clone, Default)]
pub(crate) struct TestApiV1 {
	pub(crate) initialized: bool,
	pub(crate) current_state: BTreeMap<String, ByteString>,
}

#[cfg(test)]
impl TestApiV1 {
	pub(crate) fn initialized() -> Self {
		Self { initialized: true, ..Default::default() }
	}
	pub(crate) fn uninitialized() -> Self {
		Self { initialized: false, ..Default::default() }
	}
	pub(crate) fn with_current_state(self, current_state: BTreeMap<String, ByteString>) -> Self {
		Self { current_state, ..self }
	}
}

impl ProvideRuntimeApi<Block> for TestApiV1 {
	type Api = Self;

	fn runtime_api(&self) -> sp_api::ApiRef<Self::Api> {
		(*self).clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl GovernedMapIDPApi<Block> for TestApiV1 {
		fn is_initialized() -> bool {
			self.initialized
		}
		fn get_current_state() -> BTreeMap<String, ByteString> {
			self.current_state.clone()
		}
		fn get_main_chain_scripts() -> Option<MainChainScriptsV1> {
			Some(Default::default())
		}
		fn get_pallet_version() -> u32 {
			1
		}
	}
}
