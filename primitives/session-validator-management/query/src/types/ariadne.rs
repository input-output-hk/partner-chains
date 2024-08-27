use crate::types::GetRegistrationsResponseMap;
use authority_selection_inherents::filter_invalid_candidates::PermissionedCandidateDataError;
use main_chain_follower_api::candidate::RawPermissionedCandidateData;
use serde::{Deserialize, Serialize};
use sidechain_domain::{AuraPublicKey, GrandpaPublicKey, SidechainPublicKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AriadneParameters {
	pub d_parameter: DParameter,
	pub permissioned_candidates: Vec<PermissionedCandidateData>,
	pub candidate_registrations: GetRegistrationsResponseMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DParameter {
	pub num_permissioned_candidates: u16,
	pub num_ada_candidates: u16,
	pub num_eth_candidates: u16,
}

impl DParameter {
	pub fn num_registered_candidates(&self) -> u16 {
		self.num_ada_candidates + self.num_eth_candidates
	}
}

impl From<sidechain_domain::DParameter> for DParameter {
	fn from(
		sidechain_domain::DParameter {
		  	num_permissioned_candidates,
			num_ada_candidates,
			num_eth_candidates,
		}: sidechain_domain::DParameter,
	) -> Self {
		Self { num_permissioned_candidates, num_ada_candidates, num_eth_candidates }
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionedCandidateData {
	pub sidechain_public_key: SidechainPublicKey,
	pub aura_public_key: AuraPublicKey,
	pub grandpa_public_key: GrandpaPublicKey,
	pub is_valid: bool,
	/// Human-readable reasons of registration being invalid. Present only for invalid entries.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub invalid_reasons: Option<PermissionedCandidateDataError>,
}

impl PermissionedCandidateData {
	pub fn new(
		data: RawPermissionedCandidateData,
		invalid_reasons: Option<PermissionedCandidateDataError>,
	) -> Self {
		Self {
			sidechain_public_key: data.sidechain_public_key,
			aura_public_key: data.aura_public_key,
			grandpa_public_key: data.grandpa_public_key,
			is_valid: invalid_reasons.is_none(),
			invalid_reasons,
		}
	}
}
