use crate::DataSourceError::DatumDecodeError;
use authority_selection_inherents::authority_selection_inputs::RawPermissionedCandidateData;
use cardano_serialization_lib::{PlutusData, PlutusList};
use sidechain_domain::*;

pub enum PermissionedCandidateDatums {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0(Vec<PermissionedCandidateDatumV0>),
}

pub struct PermissionedCandidateDatumV0 {
	sidechain_public_key: SidechainPublicKey,
	aura_public_key: AuraPublicKey,
	grandpa_public_key: GrandpaPublicKey,
}

impl TryFrom<PlutusData> for PermissionedCandidateDatums {
	type Error = super::Error;
	fn try_from(datum: PlutusData) -> super::Result<Self> {
		Ok(PermissionedCandidateDatums::V0(decode_legacy_permissioned_candidates_datums(datum)?))
	}
}

impl From<PermissionedCandidateDatumV0> for RawPermissionedCandidateData {
	fn from(datum: PermissionedCandidateDatumV0) -> Self {
		match datum {
			PermissionedCandidateDatumV0 {
				sidechain_public_key,
				aura_public_key,
				grandpa_public_key,
			} => Self { sidechain_public_key, aura_public_key, grandpa_public_key },
		}
	}
}

impl From<PermissionedCandidateDatums> for Vec<RawPermissionedCandidateData> {
	fn from(datums: PermissionedCandidateDatums) -> Self {
		match datums {
			PermissionedCandidateDatums::V0(datums) => datums.into_iter().map(From::from).collect(),
		}
	}
}

/// Parses plutus data schema that was used before datum versioning was added. Kept for backwards compatibility.
fn decode_legacy_permissioned_candidates_datums(
	datum: PlutusData,
) -> super::Result<Vec<PermissionedCandidateDatumV0>> {
	let list_datums: PlutusList = datum.as_list().ok_or(DatumDecodeError {
		datum: datum.clone(),
		to: "PermissionedCandidateDatumV0".to_string(),
	})?;

	let permissioned_candidates: Vec<PermissionedCandidateDatumV0> = list_datums
		.into_iter()
		.map(|keys_datum| match keys_datum.as_list() {
			Some(d) if d.len() == 3 => {
				let sc = d.get(0).as_bytes()?;
				let aura = d.get(1).as_bytes()?;
				let grandpa = d.get(2).as_bytes()?;
				Some(PermissionedCandidateDatumV0 {
					sidechain_public_key: SidechainPublicKey(sc),
					aura_public_key: AuraPublicKey(aura),
					grandpa_public_key: GrandpaPublicKey(grandpa),
				})
			},
			_ => None,
		})
		.collect::<Option<Vec<PermissionedCandidateDatumV0>>>()
		.ok_or_else(|| {
			log::error!("Could not decode {:?} to Permissioned candidates datum. Expected [[ByteString, ByteString, ByteString]].", datum.clone());
			DatumDecodeError {
				datum: datum.clone(),
				to: "PermissionedCandidateDatumV0".to_string(),
			}
		})?;

	Ok(permissioned_candidates)
}
