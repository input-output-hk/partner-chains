use crate::Result;
use async_trait::async_trait;
use serde::Serialize;
use sidechain_domain::*;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct RawPermissionedCandidateData {
	pub sidechain_public_key: SidechainPublicKey,
	pub aura_public_key: AuraPublicKey,
	pub grandpa_public_key: GrandpaPublicKey,
}

// Minotaur
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct AriadneParameters {
	pub d_parameter: DParameter,
	pub permissioned_candidates: Vec<RawPermissionedCandidateData>,
}

/// Queries about the Authority Candidates
#[async_trait]
pub trait CandidateDataSource {
	/// Returns D-parameter and list of permissioned candidates that is effective for the given epoch.
	/// The data from the latest block of `data_epoch(epoch)` will be used if available, otherwise returns data at the latest block of the chain.
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
		d_parameter: PolicyId,
		permissioned_candidates: PolicyId,
	) -> Result<AriadneParameters>;

	/// Returns the list of registrations that is effective for the given epoch.
	/// The data from the latest block of `data_epoch(epoch)` will be used if available, otherwise returns data at the latest block of the chain.
	/// Each item is a list of one candidate registrations.
	async fn get_candidates(
		&self,
		epoch: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>>;

	/// Returns Cardano Epoch Nonce. None, if the nonce for given epoch is not known yet.
	async fn get_epoch_nonce(&self, epoch: McEpochNumber) -> Result<Option<EpochNonce>>;

	///
	/// # Arguments
	///
	/// * `for_epoch`: main chain epoch number during which candidate data is meant to be used
	///
	/// returns: Result<McEpochNumber, DataSourceError> - data source methods called with `for_epoch` will query only for data which was stored on main chain in the returned epoch or earlier
	///
	///
	async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber>;
}
