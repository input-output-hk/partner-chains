use authority_selection_inherents::authority_selection_inputs::RawPermissionedCandidateData;
use partner_chains_plutus_data::permissioned_candidates::PermissionedCandidateDatums;
use sidechain_domain::*;

pub(crate) fn raw_permissioned_candidate_data_vec_from(
	datums: PermissionedCandidateDatums,
) -> Vec<RawPermissionedCandidateData> {
	Vec::<PermissionedCandidateData>::from(datums)
		.into_iter()
		.map(Into::into)
		.collect()
}
