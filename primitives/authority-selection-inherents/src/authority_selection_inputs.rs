use parity_scale_codec::{Decode, Encode};
use plutus::*;
use scale_info::TypeInfo;
use sidechain_domain::{
	CandidateRegistrations, DParameter, EpochNonce, PermissionedCandidateData, PolicyId,
};

#[cfg(feature = "std")]
use {
	main_chain_follower_api::candidate::CandidateDataSource,
	main_chain_follower_api::DataSourceError, sidechain_domain::McEpochNumber,
};

/// The part of data for selection of authorities that comes from the main chains.
/// It is unfiltered, so the selection algorithm should filter out invalid candidates.
/// Note, this is part of the state storage (see pallet_session_validator_management).
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct AuthoritySelectionInputs {
	pub d_parameter: DParameter,

	/// List of permissioned candidates from the side chain, one for each sidechain_public_key.
	pub permissioned_candidates: Vec<PermissionedCandidateData>,

	// TODO ETH: The items in the Vec should be uniquely identified by sidechain_public_key
	// (instead of  mainchain_public_key). This is necessary to allow one PC operator to register
	// stake on different chains. The registration schema is symmetric with all chains and hence
	// the same sidechain_public_key can be related to many public keys on different chains.
	// Thus:
	// - only one CandidateRegistrations per each candidate
	// - each registered_candidates[i]:
	//   - should contain valid mainchain_pub_key
    //   - may have optional eth_pub_key, in which case it may also have valid EthRegistrationData records

	/// List of registered candidates from the main chain, one for each mainchain_public_key.
	pub registered_candidates: Vec<CandidateRegistrations>,

	/// The nonce for the epoch coming from the main chain.
	pub epoch_nonce: EpochNonce,

	// TODO ETH: add nonce from Ethereum chain
}

// #[derive(Debug, PartialEq, Eq, Clone, Decode, thiserror::Error, Serialize, Deserialize)]
#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum AuthoritySelectionInputsCreationError {
	#[cfg_attr(feature = "std", error("Failed to get Ariadne parameters for epoch: {0}, D-parameter: {1:?}, permissioned candidates: {2:?}: {3}"))]
	AriadneParametersQuery(McEpochNumber, PolicyId, PolicyId, DataSourceError),
	#[cfg_attr(feature = "std", error("Failed to get registered candidates for epoch: {0}, committee candidate address: {1}: {2}."))]
	GetCandidatesQuery(McEpochNumber, String, DataSourceError),
	#[cfg_attr(feature = "std", error("Failed to get epoch nonce for epoch: {0}: {1}."))]
	GetEpochNonceQuery(McEpochNumber, DataSourceError),
}

impl AuthoritySelectionInputs {
	#[cfg(feature = "std")]
	pub async fn from_mc_data(
		candidate_data_source: &(dyn CandidateDataSource + Send + Sync),
		for_epoch: McEpochNumber,
		scripts: sp_session_validator_management::MainChainScripts,
	) -> Result<Self, AuthoritySelectionInputsCreationError> {
		let ariadne_parameters_response = candidate_data_source
			.get_ariadne_parameters(
				for_epoch,
				scripts.d_parameter_policy.clone(),
				scripts.permissioned_candidates_policy.clone(),
			)
			.await
			.map_err(|err| {
				AuthoritySelectionInputsCreationError::AriadneParametersQuery(
					for_epoch,
					scripts.d_parameter_policy,
					scripts.permissioned_candidates_policy,
					err,
				)
			})?;

		let d_parameter = ariadne_parameters_response.d_parameter;
		let permissioned_candidates = ariadne_parameters_response
			.permissioned_candidates
			.into_iter()
			.map(|candidate| PermissionedCandidateData {
				sidechain_public_key: candidate.sidechain_public_key,
				aura_public_key: candidate.aura_public_key,
				grandpa_public_key: candidate.grandpa_public_key,
			})
			.collect::<Vec<PermissionedCandidateData>>();

		let registered_candidates: Vec<CandidateRegistrations> = candidate_data_source
			.get_candidates(for_epoch, scripts.committee_candidate_address.clone())
			.await
			.map_err(|err| {
				AuthoritySelectionInputsCreationError::GetCandidatesQuery(
					for_epoch,
					scripts.committee_candidate_address.to_string(),
					err,
				)
			})?;
		let epoch_nonce_response =
			candidate_data_source.get_epoch_nonce(for_epoch).await.map_err(|err| {
				AuthoritySelectionInputsCreationError::GetEpochNonceQuery(for_epoch, err)
			})?;
		let epoch_nonce = epoch_nonce_response.unwrap_or(EpochNonce(vec![]));

		Ok(Self { d_parameter, permissioned_candidates, registered_candidates, epoch_nonce })
	}
}
