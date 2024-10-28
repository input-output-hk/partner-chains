use cardano_serialization_lib::{PlutusData, PlutusList};
use sidechain_domain::*;

use crate::{DataDecodingError, DecodingResult};

pub enum PermissionedCandidateDatums {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0(Vec<PermissionedCandidateDatumV0>),
}

pub struct PermissionedCandidateDatumV0 {
	pub sidechain_public_key: SidechainPublicKey,
	pub aura_public_key: AuraPublicKey,
	pub grandpa_public_key: GrandpaPublicKey,
}

impl TryFrom<PlutusData> for PermissionedCandidateDatums {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		Ok(PermissionedCandidateDatums::V0(decode_legacy_permissioned_candidates_datums(datum)?))
	}
}

/// Parses plutus data schema that was used before datum versioning was added. Kept for backwards compatibility.
fn decode_legacy_permissioned_candidates_datums(
	datum: PlutusData,
) -> DecodingResult<Vec<PermissionedCandidateDatumV0>> {
	let list_datums: PlutusList = datum.as_list().ok_or(DataDecodingError {
		datum: datum.clone(),
		to: "PermissionedCandidateDatumV0".to_string(),
		msg: "Expected a list".to_string(),
	})?;

	let permissioned_candidates: Vec<PermissionedCandidateDatumV0> = list_datums
		.into_iter()
		.map(decode_legacy_candidate_datum)
		.collect::<Option<Vec<PermissionedCandidateDatumV0>>>()
		.ok_or_else(|| {
			log::error!("Could not decode {:?} to Permissioned candidates datum. Expected [[ByteString, ByteString, ByteString]].", datum.clone());
			DataDecodingError {
				datum: datum.clone(),
				to: "PermissionedCandidateDatumV0".to_string(),
                msg: "Expected [[ByteString, ByteString, ByteString]]".to_string()
			}
		})?;

	Ok(permissioned_candidates)
}

fn decode_legacy_candidate_datum(datum: &PlutusData) -> Option<PermissionedCandidateDatumV0> {
	let datums = datum.as_list().filter(|datums| datums.len() == 3)?;

	let sc = datums.get(0).as_bytes()?;
	let aura = datums.get(1).as_bytes()?;
	let grandpa = datums.get(2).as_bytes()?;

	Some(PermissionedCandidateDatumV0 {
		sidechain_public_key: SidechainPublicKey(sc),
		aura_public_key: AuraPublicKey(aura),
		grandpa_public_key: GrandpaPublicKey(grandpa),
	})
}
