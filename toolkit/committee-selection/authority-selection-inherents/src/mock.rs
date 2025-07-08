//! Mocks for authority selection inherents
use crate::authority_selection_inputs::*;
use sidechain_domain::*;

#[derive(Clone)]
/// Mock implementing [AuthoritySelectionDataSource]
pub struct MockAuthoritySelectionDataSource {
	/// Registered candidates that are returned when queried for an epoch equal to its index.
	/// For example `candidates[0]` is the list of candidates that will be returned for epoch 0.
	pub candidates: Vec<Vec<CandidateRegistrations>>,
	/// Permissioned candidates that are returned when queried for an epoch equal to its index.
	/// For example `permissioned_candidates[0]` is the list of permissioned_candidates that will be returned for epoch 0.
	/// If an index is set to `None` it means that a permissioned candidates list is not set in that epoch.
	pub permissioned_candidates: Vec<Option<Vec<PermissionedCandidateData>>>,
	/// Number of permissioned candidates.
	pub num_permissioned_candidates: u16,
}

impl Default for MockAuthoritySelectionDataSource {
	fn default() -> Self {
		Self {
			candidates: vec![vec![], vec![]],
			permissioned_candidates: vec![Some(vec![]), Some(vec![])],
			num_permissioned_candidates: 3,
		}
	}
}

impl MockAuthoritySelectionDataSource {
	/// Sets registered candidates per epoch for [MockAuthoritySelectionDataSource].
	pub fn with_candidates_per_epoch(self, candidates: Vec<Vec<CandidateRegistrations>>) -> Self {
		Self { candidates, ..self }
	}

	/// Sets permissioned candidates per epoch for [MockAuthoritySelectionDataSource].
	pub fn with_permissioned_candidates(
		self,
		permissioned_candidates: Vec<Option<Vec<PermissionedCandidateData>>>,
	) -> Self {
		Self { permissioned_candidates, ..self }
	}

	/// Sets number of permissioned candidates
	pub fn with_num_permissioned_candidates(self, num_permissioned_candidates: u16) -> Self {
		Self { num_permissioned_candidates, ..self }
	}
}

#[async_trait::async_trait]
impl AuthoritySelectionDataSource for MockAuthoritySelectionDataSource {
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
		_d_parameter_policy: PolicyId,
		_permissioned_candidates_policy: PolicyId,
	) -> Result<AriadneParameters, Box<dyn std::error::Error + Send + Sync>> {
		match self.permissioned_candidates.get(epoch_number.0 as usize) {
			Some(candidates) => Ok(AriadneParameters {
				d_parameter: DParameter {
					num_permissioned_candidates: self.num_permissioned_candidates,
					num_registered_candidates: 2,
				},
				permissioned_candidates: candidates.clone(),
			}),
			_ => Err(format!("mock was called with unexpected argument: {}", epoch_number).into()),
		}
	}

	async fn get_candidates(
		&self,
		epoch_number: McEpochNumber,
		_committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self.candidates.get(epoch_number.0 as usize).cloned().unwrap_or(vec![]))
	}

	async fn get_epoch_nonce(
		&self,
		_epoch: McEpochNumber,
	) -> Result<Option<EpochNonce>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(Some(EpochNonce(vec![42u8])))
	}

	async fn data_epoch(
		&self,
		for_epoch: McEpochNumber,
	) -> Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>> {
		Ok(for_epoch)
	}
}
