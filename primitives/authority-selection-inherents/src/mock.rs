use crate::authority_selection_inputs::*;
use main_chain_follower_api::DataSourceError::{self, ExpectedDataNotFound};
use sidechain_domain::*;

#[derive(Clone)]
pub struct MockAuthoritySelectionDataSource {
	/// Each entry in each field is returned when queried for epoch equal to its index.
	/// For example `candidates[0]` is the list of candidates that will be returned for epoch 0.
	/// `candidates[1]` is the list of candidates that will be returned for epoch 1 and so on.
	pub candidates: Vec<Vec<CandidateRegistrations>>,
	pub permissioned_candidates: Vec<Option<Vec<RawPermissionedCandidateData>>>,
}

impl Default for MockAuthoritySelectionDataSource {
	fn default() -> Self {
		Self {
			candidates: vec![vec![], vec![]],
			permissioned_candidates: vec![Some(vec![]), Some(vec![])],
		}
	}
}

#[async_trait::async_trait]
impl AuthoritySelectionDataSource for MockAuthoritySelectionDataSource {
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
		_d_parameter_policy: PolicyId,
		_permissioned_candidates_policy: PolicyId,
	) -> Result<AriadneParameters, DataSourceError> {
		match self.permissioned_candidates.get(epoch_number.0 as usize) {
			Some(Some(candidates)) => Ok(AriadneParameters {
				d_parameter: DParameter {
					num_permissioned_candidates: 3,
					num_registered_candidates: 2,
				},
				permissioned_candidates: candidates.clone(),
			}),
			_ => Err(ExpectedDataNotFound("mock was called with unexpected argument".to_string())),
		}
	}

	async fn get_candidates(
		&self,
		epoch_number: McEpochNumber,
		_committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>, DataSourceError> {
		Ok(self.candidates.get(epoch_number.0 as usize).cloned().unwrap_or(vec![]))
	}

	async fn get_epoch_nonce(
		&self,
		_epoch: McEpochNumber,
	) -> Result<Option<EpochNonce>, DataSourceError> {
		Ok(Some(EpochNonce(vec![42u8])))
	}

	async fn data_epoch(&self, for_epoch: McEpochNumber) -> Result<McEpochNumber, DataSourceError> {
		Ok(for_epoch)
	}
}
