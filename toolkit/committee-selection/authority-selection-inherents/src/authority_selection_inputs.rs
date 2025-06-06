//! Types for authority selection
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use plutus::*;
use scale_info::TypeInfo;
use sidechain_domain::*;

/// The part of data for selection of authorities that comes from the main chain.
/// It is unfiltered, so the selection algorithm should filter out invalid candidates.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct AuthoritySelectionInputs {
	/// D-parameter for Ariadne committee selection. See [DParameter] for details.
	pub d_parameter: DParameter,
	/// List of permissioned candidates for committee selection.
	pub permissioned_candidates: Vec<PermissionedCandidateData>,
	/// List of registered candidates for committee selection
	pub registered_candidates: Vec<CandidateRegistrations>,
	/// Nonce for queried epoch.
	pub epoch_nonce: EpochNonce,
}

#[cfg(feature = "std")]
#[derive(Debug, thiserror::Error)]
/// Error type for creation of [AuthoritySelectionInputs]
pub enum AuthoritySelectionInputsCreationError {
	#[cfg_attr(
		feature = "std",
		error(
			"Failed to get Ariadne parameters for epoch: {0}, D-parameter: {1:?}, permissioned candidates: {2:?}: {3}"
		)
	)]
	/// Failed to get Ariadne parameters for epoch
	AriadneParametersQuery(
		McEpochNumber,
		PolicyId,
		PolicyId,
		Box<dyn std::error::Error + Send + Sync>,
	),
	#[cfg_attr(
		feature = "std",
		error(
			"Failed to get registered candidates for epoch: {0}, committee candidate address: {1}: {2}."
		)
	)]
	/// Failed to get registered candidates for epoch
	GetCandidatesQuery(McEpochNumber, String, Box<dyn std::error::Error + Send + Sync>),
	#[cfg_attr(feature = "std", error("Failed to get epoch nonce for epoch: {0}: {1}."))]
	/// Failed to get epoch nonce for epoch
	GetEpochNonceQuery(McEpochNumber, Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
/// Permissioned candidate data from Cardano main chain
pub struct RawPermissionedCandidateData {
	/// Unvalidated Partner Chain public key of permissioned candidate
	pub sidechain_public_key: SidechainPublicKey,
	/// Unvalidated Aura public key of permissioned candidate
	pub aura_public_key: AuraPublicKey,
	/// Unvalidated Beefy public key of permissioned candidate
	pub beefy_public_key: BeefyPublicKey,
	/// Unvalidated Grandpa public key of permissioned candidate
	pub grandpa_public_key: GrandpaPublicKey,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
/// Ariadne selection algorithm parameters owned by the Partner Chain Governance Authority.
pub struct AriadneParameters {
	/// D-parameter for Ariadne committee selection. See [DParameter] for details.
	pub d_parameter: DParameter,
	/// List of permissioned candidates for committee selection.
	/// [None] means that a list of permissioned candidates has not been set on the mainchain.
	pub permissioned_candidates: Option<Vec<RawPermissionedCandidateData>>,
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
	pub(crate) async fn from_mc_data(
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
					scripts.d_parameter_policy_id.clone(),
					scripts.permissioned_candidates_policy_id.clone(),
					err,
				)
			})?;

		let d_parameter = ariadne_parameters_response.d_parameter;
		let no_permissioned_candidates_expected = d_parameter.num_permissioned_candidates == 0;
		let permissioned_candidates = match ariadne_parameters_response.permissioned_candidates {
			None if no_permissioned_candidates_expected => Vec::new(),
			None => {
				return Err(AuthoritySelectionInputsCreationError::AriadneParametersQuery(
					for_epoch,
					scripts.d_parameter_policy_id,
					scripts.permissioned_candidates_policy_id,
					("Expected Data Not Found: Permissioned Candidates List".to_string()).into(),
				));
			},
			Some(permissioned_candidates) => permissioned_candidates
				.into_iter()
				.map(|candidate| PermissionedCandidateData {
					sidechain_public_key: candidate.sidechain_public_key,
					aura_public_key: candidate.aura_public_key,
					beefy_public_key: candidate.beefy_public_key,
					grandpa_public_key: candidate.grandpa_public_key,
				})
				.collect::<Vec<PermissionedCandidateData>>(),
		};

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
