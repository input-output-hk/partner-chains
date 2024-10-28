use cardano_serialization_lib::{PlutusData, PlutusList};
use sidechain_domain::*;

use crate::{DataDecodingError, DecodingResult};

#[derive(Clone, Debug, PartialEq)]
pub enum PermissionedCandidateDatums {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0(Vec<PermissionedCandidateDatumV0>),
}

#[derive(Clone, Debug, PartialEq)]
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;

	#[test]
	fn valid_permissioned_candidates_1() {
		let plutus_data = test_plutus_data!({"list": [
			{"list": [{"bytes": "bb11"}, {"bytes": "cc11"}, {"bytes": "dd11"}]},
			{"list": [{"bytes": "bb22"}, {"bytes": "cc22"}, {"bytes": "dd22"}]}
		]});

		let expected_datum = PermissionedCandidateDatums::V0(vec![
			PermissionedCandidateDatumV0 {
				sidechain_public_key: SidechainPublicKey(hex!("bb11").into()),
				aura_public_key: AuraPublicKey(hex!("cc11").into()),
				grandpa_public_key: GrandpaPublicKey(hex!("dd11").into()),
			},
			PermissionedCandidateDatumV0 {
				sidechain_public_key: SidechainPublicKey(hex!("bb22").into()),
				aura_public_key: AuraPublicKey(hex!("cc22").into()),
				grandpa_public_key: GrandpaPublicKey(hex!("dd22").into()),
			},
		]);

		assert_eq!(PermissionedCandidateDatums::try_from(plutus_data).unwrap(), expected_datum)
	}

	#[test]
	fn valid_permissioned_candidates_2() {
		let plutus_data = test_plutus_data!({"list": [
			{"list": [
				{"bytes": "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"},
				{"bytes": "bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec"},
				{"bytes": "9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d"}
			]},
			{"list": [
				{"bytes": "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"},
				{"bytes": "56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19"},
				{"bytes": "7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32"}
			]}
		]});

		let expected_datum = PermissionedCandidateDatums::V0(vec![
			PermissionedCandidateDatumV0 {
				sidechain_public_key: SidechainPublicKey(
					hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854").into(),
				),
				aura_public_key: AuraPublicKey(
					hex!("bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec").into(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d").into(),
				),
			},
			PermissionedCandidateDatumV0 {
				sidechain_public_key: SidechainPublicKey(
					hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf").into(),
				),
				aura_public_key: AuraPublicKey(
					hex!("56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19").into(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32").into(),
				),
			},
		]);

		assert_eq!(PermissionedCandidateDatums::try_from(plutus_data).unwrap(), expected_datum)
	}
}
