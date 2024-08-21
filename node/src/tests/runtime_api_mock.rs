use super::mock::mock_sidechain_params;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use chain_params::SidechainParams;
use sidechain_domain::*;
use sidechain_mc_hash::McHashInherentDigest;
use sidechain_runtime::opaque::SessionKeys;
use sidechain_runtime::CrossChainPublic;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_core::ecdsa;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor, Zero};
use sp_runtime::Digest;
use sp_sidechain::GetSidechainParams;
use std::collections::HashMap;

type Hash = <Block as BlockT>::Hash;
type Header = <Block as BlockT>::Header;

#[derive(Clone)]
pub struct TestApi {
	pub next_unset_epoch_number: ScEpochNumber,
	pub headers: HashMap<Hash, Header>,
}

impl TestApi {
	pub fn new(next_unset_epoch_number: ScEpochNumber) -> Self {
		let header = mock_header();
		let mut headers = HashMap::new();
		headers.insert(header.hash(), header);
		Self { next_unset_epoch_number, headers }
	}

	pub fn with_headers<Hs: Into<HashMap<Hash, Header>>>(self, headers: Hs) -> Self {
		Self { headers: headers.into(), ..self }
	}
}

impl Default for TestApi {
	fn default() -> Self {
		Self::new(ScEpochNumber(2))
	}
}

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = TestApi;

	fn runtime_api(&self) -> ApiRef<Self::Api> {
		self.clone().into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl GetSidechainParams<Block, SidechainParams> for TestApi {
		fn sidechain_params() -> SidechainParams { mock_sidechain_params() }
	}

	impl sp_session_validator_management::SessionValidatorManagementApi<Block, SessionKeys, CrossChainPublic, AuthoritySelectionInputs, ScEpochNumber> for TestApi {
		fn get_current_committee() -> (ScEpochNumber, Vec<CrossChainPublic>) {
			unimplemented!()
		}
		fn get_next_unset_epoch_number() -> sidechain_domain::ScEpochNumber {
			self.next_unset_epoch_number
		}
		fn calculate_committee(authority_selection_inputs: AuthoritySelectionInputs, _sidechain_epoch: sidechain_domain::ScEpochNumber) -> Option<Vec<(CrossChainPublic, SessionKeys)>> {
			if authority_selection_inputs.registered_candidates.is_empty() {
				None
			} else {
				let result: Vec<(CrossChainPublic, SessionKeys)> = authority_selection_inputs.registered_candidates.into_iter().map(|candidate| {
					let registration = candidate.registrations().first().unwrap().clone();
					let cross_chain_pub_slice: [u8; 33] = registration.cross_chain_pub_key.0.try_into().unwrap();
					let cross_chain_public: CrossChainPublic = CrossChainPublic::from(ecdsa::Public::from(cross_chain_pub_slice));
					let aura_pub_key = registration.aura_pub_key.try_into_sr25519().unwrap();
					let grandpa_pub_key = registration.grandpa_pub_key.try_into_ed25519().unwrap();
					let session_keys = (aura_pub_key, grandpa_pub_key).into();
					(cross_chain_public, session_keys)
				}).collect();
				Some(result)
			}
		}
		fn get_main_chain_scripts() -> sp_session_validator_management::MainChainScripts {
			sp_session_validator_management::MainChainScripts {
				committee_candidate_address: MainchainAddress::default(),
				d_parameter_policy_id: PolicyId::default(),
				permissioned_candidates_policy_id: PolicyId::default(),
			}
		}
	}

	impl sp_native_token_management::NativeTokenManagementApi<Block> for TestApi {
		fn get_main_chain_scripts() -> sp_native_token_management::MainChainScripts {
			sp_native_token_management::MainChainScripts {
				native_token_policy_id: Default::default(),
				native_token_asset_name: Default::default(),
				illiquid_supply_validator_address: Default::default(),

			}
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

	fn info(&self) -> sc_client_api::blockchain::Info<Block> {
		sc_client_api::blockchain::Info {
			best_hash: Default::default(),
			best_number: Zero::zero(),
			finalized_hash: Default::default(),
			finalized_number: Zero::zero(),
			genesis_hash: Default::default(),
			number_leaves: Default::default(),
			finalized_state: None,
			block_gap: None,
		}
	}

	fn status(
		&self,
		_id: <Block as BlockT>::Hash,
	) -> Result<sc_client_api::blockchain::BlockStatus, sp_blockchain::Error> {
		Ok(sc_client_api::blockchain::BlockStatus::Unknown)
	}

	fn number(
		&self,
		_hash: <Block as BlockT>::Hash,
	) -> Result<Option<NumberFor<Block>>, sp_blockchain::Error> {
		Ok(None)
	}

	fn hash(
		&self,
		_number: NumberFor<Block>,
	) -> Result<Option<<Block as BlockT>::Hash>, sp_blockchain::Error> {
		unimplemented!()
	}
}

pub fn mock_header() -> <Block as BlockT>::Header {
	<Block as BlockT>::Header::new(
		Default::default(),
		Default::default(),
		Default::default(),
		Default::default(),
		Digest { logs: McHashInherentDigest::from_mc_block_hash(McBlockHash([1; 32])) },
	)
}
