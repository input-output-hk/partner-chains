use crate::authority_selection_inputs::AuthoritySelectionInputs;
use mock_types::session_keys::SessionKeys;
use sidechain_domain::*;
use sp_session_validator_management::{MainChainScripts, SessionValidatorManagementApi};

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

#[derive(Clone)]
pub struct TestApi {
	pub next_unset_epoch_number: ScEpochNumber,
}

impl sp_api::ProvideRuntimeApi<Block> for TestApi {
	type Api = TestApi;

	fn runtime_api(&self) -> sp_api::ApiRef<Self::Api> {
		self.clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl SessionValidatorManagementApi<Block, SessionKeys, CrossChainPublicKey, AuthoritySelectionInputs, ScEpochNumber>
		for TestApi
	{
		fn get_current_committee() -> (ScEpochNumber, Vec<CrossChainPublicKey>) {
			unimplemented!()
		}
		fn get_next_unset_epoch_number() -> sidechain_domain::ScEpochNumber {
			self.next_unset_epoch_number
		}
		fn calculate_committee(
			_authority_selection_inputs: AuthoritySelectionInputs,
			_sidechain_epoch: sidechain_domain::ScEpochNumber,
		) -> Option<Vec<(CrossChainPublicKey, SessionKeys)>> {
			unimplemented!()
		}
		fn get_main_chain_scripts() -> MainChainScripts {
			MainChainScripts {
				committee_candidate_address: MainchainAddress::default(),
				d_parameter_policy_id: PolicyId::default(),
				permissioned_candidates_policy_id: PolicyId::default(),
			}
		}
	}
}
