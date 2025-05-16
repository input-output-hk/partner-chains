//! Queries for committee selection
#![deny(missing_docs)]
pub mod commands;
pub mod get_registrations;
pub mod types;

use async_trait::async_trait;
use authority_selection_inherents::authority_selection_inputs::{
	AuthoritySelectionDataSource, AuthoritySelectionInputs, RawPermissionedCandidateData,
};
use authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi;
use derive_new::new;
use parity_scale_codec::{Decode, Encode};
use sidechain_block_search::{FindSidechainBlock, SidechainInfo, predicates::AnyBlockInEpoch};
use sidechain_domain::{McEpochNumber, ScEpochNumber, StakePoolPublicKey};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::{HeaderBackend, Info};
use sp_core::bytes::to_hex;
use sp_runtime::traits::NumberFor;
use sp_runtime::traits::{Block as BlockT, Zero};
use sp_session_validator_management::{
	CommitteeMember as CommitteeMemberT, SessionValidatorManagementApi,
};
use sp_sidechain::{GetGenesisUtxo, GetSidechainStatus};
use std::sync::Arc;
use types::*;

#[cfg(test)]
mod tests;

/// Result type for queries
pub type QueryResult<T> = Result<T, String>;

#[async_trait]
/// API for Session Validator Management Queries
pub trait SessionValidatorManagementQueryApi {
	/// Returns the committee for given sidechain epoch. The order of the list represents the order of slot allocation.
	fn get_epoch_committee(&self, epoch_number: u64) -> QueryResult<GetCommitteeResponse>;

	///
	/// returns: Last active and valid registration followed by all newer invalid registrations for mc_epoch_number and stake_pool_public_key.
	/// Regardless of `mc_epoch_number` value, it always uses validation api from the latest sidechain block.
	///
	async fn get_registrations(
		&self,
		mc_epoch_number: McEpochNumber,
		stake_pool_public_key: StakePoolPublicKey,
	) -> QueryResult<Vec<CandidateRegistrationEntry>>;

	/// Regardless of `epoch_number` value, all the candidates data validation is done based on the validation api from the latest sidechain block.
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> QueryResult<AriadneParameters>;
}

#[derive(new)]
/// Session Validator Management Query type wrapping client, and data source
pub struct SessionValidatorManagementQuery<C, Block, CommitteeMember: Decode> {
	client: Arc<C>,
	candidate_data_source: Arc<dyn AuthoritySelectionDataSource + Send + Sync>,
	_marker: std::marker::PhantomData<(Block, CommitteeMember)>,
}

impl<C, Block, CommitteeMember> SessionValidatorManagementQuery<C, Block, CommitteeMember>
where
	Block: BlockT,
	C: ProvideRuntimeApi<Block>,
	C::Api: sp_api::Core<Block> + ApiExt<Block>,
	CommitteeMember: CommitteeMemberT + Encode + Decode,
	CommitteeMember::AuthorityId: Encode + Decode + AsRef<[u8]>,
	CommitteeMember::AuthorityKeys: Encode + Decode,
	AuthoritySelectionInputs: Encode + Decode,
	C::Api: SessionValidatorManagementApi<
			Block,
			CommitteeMember,
			AuthoritySelectionInputs,
			ScEpochNumber,
		>,
{
	fn validator_management_api_version(&self, block: Block::Hash) -> QueryResult<u32> {
		let version = (self.client.runtime_api())
			.api_version::<dyn SessionValidatorManagementApi<
					Block,
					CommitteeMember,
					AuthoritySelectionInputs,
					ScEpochNumber,
				>>(block)
			.map_err(err_debug)?
			.unwrap_or(1);
		Ok(version)
	}

	fn get_current_committee_versioned(
		&self,
		block: Block::Hash,
	) -> QueryResult<GetCommitteeResponse> {
		let api = self.client.runtime_api();

		if self.validator_management_api_version(block)? < 2 {
			#[allow(deprecated)]
			let (epoch, authority_ids) =
				api.get_current_committee_before_version_2(block).map_err(err_debug)?;
			Ok(GetCommitteeResponse::new_legacy(epoch, authority_ids))
		} else {
			let (epoch, authority_data) = api.get_current_committee(block).map_err(err_debug)?;
			Ok(GetCommitteeResponse::new(epoch, authority_data))
		}
	}

	fn get_next_committee_versioned(
		&self,
		block: Block::Hash,
	) -> QueryResult<Option<GetCommitteeResponse>> {
		let api = self.client.runtime_api();

		if self.validator_management_api_version(block)? < 2 {
			#[allow(deprecated)]
			Ok(api.get_next_committee_before_version_2(block).map_err(err_debug)?.map(
				|(epoch, authority_ids)| GetCommitteeResponse::new_legacy(epoch, authority_ids),
			))
		} else {
			Ok(api
				.get_next_committee(block)
				.map_err(err_debug)?
				.map(|(epoch, authority_data)| GetCommitteeResponse::new(epoch, authority_data)))
		}
	}
}

#[async_trait]
impl<C, Block, CommitteeMember> SessionValidatorManagementQueryApi
	for SessionValidatorManagementQuery<C, Block, CommitteeMember>
where
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
	CommitteeMember: CommitteeMemberT + Decode + Encode + Send + Sync + 'static,
	CommitteeMember::AuthorityKeys: Decode + Encode,
	CommitteeMember::AuthorityId: AsRef<[u8]> + Decode + Encode + Send + Sync + 'static,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block>,
	C::Api: sp_api::Core<Block>,
	C::Api: GetSidechainStatus<Block>,
	C::Api: SessionValidatorManagementApi<
			Block,
			CommitteeMember,
			AuthoritySelectionInputs,
			ScEpochNumber,
		>,
	C::Api: GetGenesisUtxo<Block>,
	C::Api: CandidateValidationApi<Block>,
{
	fn get_epoch_committee(&self, epoch_number: u64) -> QueryResult<GetCommitteeResponse> {
		let epoch_number = ScEpochNumber(epoch_number);
		let Info { genesis_hash, best_number: latest_block, best_hash, .. } = self.client.info();

		if epoch_number.is_zero() {
			let genesis_committee = self.get_current_committee_versioned(genesis_hash)?;
			return Ok(GetCommitteeResponse { sidechain_epoch: 0, ..genesis_committee });
		}

		let first_epoch = {
			let second_block = (self.client)
				.hash(1.into())
				.map_err(|err| {
					format!("Node is not in archive mode, not able to fetch first block: {err:?}")
				})?
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

		if epoch_number == epoch_of_latest_block.next() {
			self.get_next_committee_versioned(best_hash)?
				.ok_or(format!("Committee is unknown for the next epoch: {epoch_number}"))
		} else {
			let block_hash = self
				.client
				.find_block(AnyBlockInEpoch { epoch: epoch_number })
				.map_err(err_debug)?;
			self.get_current_committee_versioned(block_hash)
		}
	}

	async fn get_registrations(
		&self,
		mc_epoch_number: McEpochNumber,
		mc_public_key: StakePoolPublicKey,
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

		let permissioned_candidates = match ariadne_parameters_response.permissioned_candidates {
			None => None,
			Some(permissioned_candidates) => Some(
				permissioned_candidates
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
			),
		};

		Ok(AriadneParameters {
			d_parameter: ariadne_parameters_response.d_parameter.into(),
			permissioned_candidates,
			candidate_registrations,
		})
	}
}

fn err_debug<T: std::fmt::Debug>(err: T) -> String {
	format!("{err:?}")
}
