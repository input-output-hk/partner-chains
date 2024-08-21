pub mod commands;
pub mod get_registrations;
pub mod types;

use async_trait::async_trait;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi;
use derive_new::new;
use main_chain_follower_api::candidate::RawPermissionedCandidateData;
use main_chain_follower_api::CandidateDataSource;
use plutus::ToDatum;
use sidechain_block_search::{predicates::AnyBlockInEpoch, FindSidechainBlock, SidechainInfo};
use sidechain_domain::{MainchainPublicKey, McEpochNumber, ScEpochNumber};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::{HeaderBackend, Info};
use sp_core::bytes::to_hex;
use sp_runtime::traits::NumberFor;
use sp_runtime::traits::{Block as BlockT, Zero};
use sp_session_validator_management::SessionValidatorManagementApi;
use sp_sidechain::{GetSidechainParams, GetSidechainStatus};
use std::sync::Arc;
use types::*;

pub type QueryResult<T> = Result<T, String>;

#[async_trait]
pub trait SessionValidatorManagementQueryApi {
	/// Returns the committee for given sidechain epoch. The order of the list represents the order of slot allocation.
	fn get_epoch_committee(&self, epoch_number: u64) -> QueryResult<GetCommitteeResponse>;

	///
	/// returns: Last active and valid registration followed by all newer invalid registrations for mc_epoch_number and mc_public_key.
	/// Regardless of `mc_epoch_number` value, it always uses validation api from the latest sidechain block.
	///
	async fn get_registrations(
		&self,
		mc_epoch_number: McEpochNumber,
		mc_public_key: MainchainPublicKey,
	) -> QueryResult<Vec<CandidateRegistrationEntry>>;

	/// Regardless of `epoch_number` value, all the candidates data validation is done based on the validation api from the latest sidechain block.
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> QueryResult<AriadneParameters>;
}

#[derive(new)]
pub struct SessionValidatorManagementQuery<
	C,
	Block,
	SessionKeys: parity_scale_codec::Decode,
	CrossChainPublic,
	SidechainParams: parity_scale_codec::Decode + ToDatum + Clone + Send + Sync + 'static,
> {
	client: Arc<C>,
	candidate_data_source: Arc<dyn CandidateDataSource + Send + Sync>,
	_marker: std::marker::PhantomData<(Block, SessionKeys, CrossChainPublic, SidechainParams)>,
}

#[async_trait]
impl<
		C,
		Block,
		SessionKeys: parity_scale_codec::Decode + Send + Sync + 'static,
		SidechainParams: parity_scale_codec::Decode + ToDatum + Clone + Send + Sync + 'static,
		CrossChainPublic: parity_scale_codec::Decode
			+ parity_scale_codec::Encode
			+ AsRef<[u8]>
			+ Send
			+ Sync
			+ 'static,
	> SessionValidatorManagementQueryApi
	for SessionValidatorManagementQuery<C, Block, SessionKeys, CrossChainPublic, SidechainParams>
where
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: sp_api::Core<Block>,
	C::Api: GetSidechainStatus<Block>,
	C::Api: SessionValidatorManagementApi<
		Block,
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	C::Api: GetSidechainParams<Block, SidechainParams>,
	C::Api: CandidateValidationApi<Block>,
{
	fn get_epoch_committee(&self, epoch_number: u64) -> QueryResult<GetCommitteeResponse> {
		let epoch_number = ScEpochNumber(epoch_number);
		let Info { genesis_hash, best_number: latest_block, best_hash, .. } = self.client.info();

		if epoch_number.is_zero() {
			let (_, genesis_committee) = (self.client.runtime_api())
				.get_current_committee(genesis_hash)
				.map_err(err_debug)?;
			return Ok(GetCommitteeResponse::new(epoch_number, genesis_committee));
		}

		let first_epoch = {
			let second_block = (self.client)
				.hash(1.into())
				.map_err(err_debug)?
				.ok_or("Only the Genesis Block exists at the moment!")?;
			(self.client.runtime_api())
				.get_sidechain_status(second_block)
				.map_err(err_debug)?
				.epoch
		};

		if epoch_number < first_epoch {
			return Err(format!("Epoch {} is earlier than the Initial Epoch!", epoch_number));
		}

		let epoch_of_latest_block =
			self.client.get_epoch_of_block(latest_block).map_err(err_debug)?;

		if epoch_number > epoch_of_latest_block.next() {
			return Err(format!("Committee is unknown for epoch {epoch_number}"));
		}

		let (_, committee) = if epoch_number == epoch_of_latest_block.next() {
			(self.client.runtime_api())
				.get_next_committee(best_hash)
				.map_err(err_debug)?
				.ok_or(format!("Committee is unknown for the next epoch: {epoch_number}"))
		} else {
			let block_hash = self
				.client
				.find_block(AnyBlockInEpoch { epoch: epoch_number })
				.map_err(err_debug)?;
			(self.client.runtime_api()).get_current_committee(block_hash).map_err(err_debug)
		}?;

		Ok(GetCommitteeResponse::new(epoch_number, committee))
	}

	async fn get_registrations(
		&self,
		mc_epoch_number: McEpochNumber,
		mc_public_key: MainchainPublicKey,
	) -> QueryResult<Vec<CandidateRegistrationEntry>> {
		let api = self.client.runtime_api();
		let best_block = self.client.info().best_hash;
		let scripts = api.get_main_chain_scripts(best_block).map_err(err_debug)?;
		let mut registrations_map = self
			.candidates_registrations_for_epoch(
				mc_epoch_number,
				scripts.committee_candidate_address,
			)
			.await?;
		Ok(registrations_map.remove(&to_hex(&mc_public_key.0, false)).unwrap_or(vec![]))
	}

	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> QueryResult<AriadneParameters> {
		let api = self.client.runtime_api();
		let best_block = self.client.info().best_hash;
		let scripts = api.get_main_chain_scripts(best_block).map_err(err_debug)?;
		let ariadne_parameters_response = self
			.candidate_data_source
			.get_ariadne_parameters(
				epoch_number,
				scripts.d_parameter_policy_id,
				scripts.permissioned_candidates_policy_id,
			)
			.await
			.map_err(err_debug)?;

		let candidate_registrations = self
			.candidates_registrations_for_epoch(epoch_number, scripts.committee_candidate_address)
			.await?;
		let validate_permissioned_candidate = |candidate: &RawPermissionedCandidateData| {
			api.validate_permissioned_candidate_data(
				best_block,
				sidechain_domain::PermissionedCandidateData {
					sidechain_public_key: candidate.sidechain_public_key.clone(),
					aura_public_key: candidate.aura_public_key.clone(),
					grandpa_public_key: candidate.grandpa_public_key.clone(),
				},
			)
		};

		Ok(AriadneParameters {
			d_parameter: ariadne_parameters_response.d_parameter.into(),
			permissioned_candidates: ariadne_parameters_response
				.permissioned_candidates
				.into_iter()
				.map(|candidate| {
					let validation_result =
						validate_permissioned_candidate(&candidate).map_err(err_debug)?;
					Ok::<PermissionedCandidateData, String>(PermissionedCandidateData::new(
						candidate,
						validation_result,
					))
				})
				.collect::<Result<Vec<_>, _>>()?,
			candidate_registrations,
		})
	}
}

pub fn err_debug<T: std::fmt::Debug>(err: T) -> String {
	format!("{err:?}")
}
