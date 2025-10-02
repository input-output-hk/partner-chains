use crate::Result;
use async_trait::async_trait;
use authority_selection_inherents::*;
use sidechain_domain::*;

pub struct AuthoritySelectionDataSourceImpl {}

impl AuthoritySelectionDataSourceImpl {
	pub fn new() -> Self {
		Self {}
	}
}

#[async_trait]
impl AuthoritySelectionDataSource for AuthoritySelectionDataSourceImpl {
	async fn get_ariadne_parameters(
		&self,
		_epoch_number: McEpochNumber,
		_d_parameter_validator: PolicyId,
		_permissioned_candidates_validator: PolicyId,
	) -> Result<AriadneParameters> {
		Err("not implemented".into())
	}

	async fn get_candidates(
		&self,
		_epoch: McEpochNumber,
		_committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>> {
		Err("not implemented".into())
	}

	async fn get_epoch_nonce(&self, _epoch_number: McEpochNumber) -> Result<Option<EpochNonce>> {
		Err("not implemented".into())
	}

	async fn data_epoch(&self, _for_epoch: McEpochNumber) -> Result<McEpochNumber> {
		Err("not implemented".into())
	}
}
