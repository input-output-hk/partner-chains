use authority_selection_inherents::authority_selection_inputs::RawPermissionedCandidateData;
use partner_chains_plutus_data::permissioned_candidates::PermissionedCandidateDatumV0;
use partner_chains_plutus_data::permissioned_candidates::PermissionedCandidateDatums;
use sidechain_domain::*;

pub fn raw_permissioned_candidate_data_from(
	datum: PermissionedCandidateDatumV0,
) -> RawPermissionedCandidateData {
	let PermissionedCandidateDatumV0 {
		sidechain_public_key,
		aura_public_key,
		grandpa_public_key,
		im_online_public_key,
	} = datum;
	RawPermissionedCandidateData {
		sidechain_public_key,
		aura_public_key,
		grandpa_public_key,
		im_online_public_key,
	}
}

pub fn raw_permissioned_candidate_data_vec_from(
	datums: PermissionedCandidateDatums,
) -> Vec<RawPermissionedCandidateData> {
	match datums {
		PermissionedCandidateDatums::V0(datums) => {
			datums.into_iter().map(raw_permissioned_candidate_data_from).collect()
		},
	}
}
