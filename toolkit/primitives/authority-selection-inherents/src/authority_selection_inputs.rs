use parity_scale_codec::{Decode, Encode};
use plutus::*;
use scale_info::TypeInfo;
use sidechain_domain::*;

/// The part of data for selection of authorities that comes from the main chain.
/// It is unfiltered, so the selection algorithm should filter out invalid candidates.
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub struct AuthoritySelectionInputs {
	pub d_parameter: DParameter,
	pub permissioned_candidates: Vec<PermissionedCandidateData>,
	pub registered_candidates: Vec<CandidateRegistrations>,
	pub epoch_nonce: EpochNonce,
}

#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
pub enum AuthoritySelectionInputsCreationError {
	#[cfg_attr(feature = "std", error("Failed to get Ariadne parameters for epoch: {0}, D-parameter: {1:?}, permissioned candidates: {2:?}: {3}"))]
	AriadneParametersQuery(
		McEpochNumber,
		PolicyId,
		PolicyId,
		Box<dyn std::error::Error + Send + Sync>,
	),
	#[cfg_attr(feature = "std", error("Failed to get registered candidates for epoch: {0}, committee candidate address: {1}: {2}."))]
	GetCandidatesQuery(McEpochNumber, String, Box<dyn std::error::Error + Send + Sync>),
	#[cfg_attr(feature = "std", error("Failed to get epoch nonce for epoch: {0}: {1}."))]
	GetEpochNonceQuery(McEpochNumber, Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RawPermissionedCandidateData {
	pub sidechain_public_key: SidechainPublicKey,
	pub aura_public_key: AuraPublicKey,
	pub grandpa_public_key: GrandpaPublicKey,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AriadneParameters {
	pub d_parameter: DParameter,
	pub permissioned_candidates: Vec<RawPermissionedCandidateData>,
}

/// Queries about the Authority Candidates
#[cfg(feature = "std")]
#[async_trait::async_trait]
pub trait AuthoritySelectionDataSource {
	/// Returns D-parameter and list of permissioned candidates that is effective for the given epoch.
	/// The data from the latest block of `data_epoch(epoch)` will be used if available, otherwise returns data at the latest block of the chain.
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
		d_parameter: PolicyId,
		permissioned_candidates: PolicyId,
	) -> Result<AriadneParameters, Box<dyn std::error::Error + Send + Sync>>;

	/// Returns the list of registrations that is effective for the given epoch.
	/// The data from the latest block of `data_epoch(epoch)` will be used if available, otherwise returns data at the latest block of the chain.
	/// Each item is a list of one candidate registrations.
	async fn get_candidates(
		&self,
		epoch: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>, Box<dyn std::error::Error + Send + Sync>>;

	/// Returns Cardano Epoch Nonce. None, if the nonce for given epoch is not known yet.
	async fn get_epoch_nonce(
		&self,
		epoch: McEpochNumber,
	) -> Result<Option<EpochNonce>, Box<dyn std::error::Error + Send + Sync>>;

	///
	/// # Arguments
	///
	/// * `for_epoch`: main chain epoch number during which candidate data is meant to be used
	///
	/// returns: Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>> - data source methods called with `for_epoch` will query only for data which was stored on main chain in the returned epoch or earlier
	///
	///
	async fn data_epoch(
		&self,
		for_epoch: McEpochNumber,
	) -> Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>>;
}

impl AuthoritySelectionInputs {
	#[cfg(feature = "std")]
	pub async fn from_mc_data(
		candidate_data_source: &(dyn AuthoritySelectionDataSource + Send + Sync),
		for_epoch: McEpochNumber,
		scripts: sp_session_validator_management::MainChainScripts,
	) -> Result<Self, AuthoritySelectionInputsCreationError> {
		let ariadne_parameters_response = candidate_data_source
			.get_ariadne_parameters(
				for_epoch,
				scripts.d_parameter_policy_id.clone(),
				scripts.permissioned_candidates_policy_id.clone(),
			)
			.await
			.map_err(|err| {
				AuthoritySelectionInputsCreationError::AriadneParametersQuery(
					for_epoch,
					scripts.d_parameter_policy_id,
					scripts.permissioned_candidates_policy_id,
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
