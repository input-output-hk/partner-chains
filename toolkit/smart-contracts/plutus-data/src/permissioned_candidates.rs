//! Plutus data types for permissioned candidates.
use cardano_serialization_lib::{BigNum, PlutusData, PlutusList};
use sidechain_domain::*;

use crate::{
	DataDecodingError, DecodingResult, VersionedDatum, VersionedDatumWithLegacy,
	VersionedGenericDatum, candidate_keys::*,
};

#[derive(Clone, Debug, PartialEq)]
/// Datum representing a list of permissioned candidates.
pub enum PermissionedCandidateDatums {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0(Vec<PermissionedCandidateDatumV0>),
	/// Schema with generic set of keys
	V1(Vec<PermissionedCandidateDatumV1>),
}

#[derive(Clone, Debug, PartialEq)]
/// Datum representing a permissioned candidate.
pub struct PermissionedCandidateDatumV0 {
	/// Sidechain public key of the trustless candidate
	pub sidechain_public_key: SidechainPublicKey,
	/// Aura public key of the trustless candidate
	pub aura_public_key: AuraPublicKey,
	/// GRANDPA public key of the trustless candidate
	pub grandpa_public_key: GrandpaPublicKey,
}

impl From<PermissionedCandidateDatumV1> for PermissionedCandidateData {
	// In the follow-up PermissionedCandidateData will become parametrized
	// with T: OpaqueKeys, this function will be re-implemented.
	fn from(value: PermissionedCandidateDatumV1) -> Self {
		let PermissionedCandidateDatumV1 { partner_chains_key, keys } = value;
		PermissionedCandidateData { sidechain_public_key: partner_chains_key, keys }
	}
}

#[derive(Clone, Debug, PartialEq)]
/// Datum representing a permissioned candidate with arbitrary set of keys
pub struct PermissionedCandidateDatumV1 {
	/// Partner Chains key identifier and bytes
	pub partner_chains_key: SidechainPublicKey,
	/// Represents arbitrary set of keys with 4 character identifier
	pub keys: CandidateKeys,
}

impl TryFrom<PlutusData> for PermissionedCandidateDatums {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		Self::decode(&datum)
	}
}

impl From<PermissionedCandidateDatumV0> for PermissionedCandidateData {
	fn from(value: PermissionedCandidateDatumV0) -> Self {
		Self {
			sidechain_public_key: value.sidechain_public_key,
			keys: CandidateKeys(vec![
				value.aura_public_key.into(),
				value.grandpa_public_key.into(),
			]),
		}
	}
}

impl From<PermissionedCandidateDatums> for Vec<PermissionedCandidateData> {
	fn from(value: PermissionedCandidateDatums) -> Self {
		match value {
			PermissionedCandidateDatums::V0(v) => v.into_iter().map(|d| d.into()).collect(),
			PermissionedCandidateDatums::V1(v) => v.into_iter().map(|d| d.into()).collect(),
		}
	}
}

/// Converts a list of [PermissionedCandidateData] values to [VersionedGenericDatum] encoded as [PlutusData].
/// Version 0 is used for specific set of Partner Chains Key: partner chains key, AURA, Grandpa
/// If other set of key is used, then version 1 is used.
/// Encoding:
/// ```ignore
///   VersionedGenericDatum:
///   - datum: Constr 0 []
///   - appendix:
///     [
///       [ candidates[0].sidechain_public_key
///       , candidates[0].aura_public_key
///       , candidates[0].grandpa_public_key
///       ]
///     ,
///       [ candidates[1].sidechain_public_key
///       , candidates[1].aura_public_key
///       , candidates[1].grandpa_public_key
///       ]
///       // etc.
///     ]
///   - version: 0
/// or:
///   VersionedGenericDatum:
///   - datum: Constr 0 []
///   - appendix:
///     [
///       [ candidates[0].sidechain_public_key
/// 	  ,
///         [
/// 	      [ candidates[0].keys[0].id,
///           , candidates[0].keys[0].bytes
///           ]
///         , [ candidates[0].keys[1].id,
///           , candidates[0].keys[1].bytes
///           ]
///           // etc.
///         ]
///       ]
///     ,
///       [ candidates[1].sidechain_public_key
/// 	  ,
///         [
/// 	      [ candidates[1].keys[0].id,
///           , candidates[1].keys[0].bytes
///           ]
///         , [ candidates[1].keys[1].id,
///           , candidates[1].keys[1].bytes
///           ]
///           // etc.
///         ]
///       ]
///       // etc.
///     ]
///   - version: 1
/// ```

pub fn permissioned_candidates_to_plutus_data(
	candidates: &[PermissionedCandidateData],
) -> PlutusData {
	fn candidates_to_plutus_data_v0(candidates: &[PermissionedCandidateData]) -> PlutusData {
		let mut list = PlutusList::new();
		for candidate in candidates {
			let mut candidate_datum = PlutusList::new();
			candidate_datum.add(&PlutusData::new_bytes(candidate.sidechain_public_key.0.clone()));
			for key in candidate.keys.0.iter() {
				candidate_datum.add(&PlutusData::new_bytes(key.bytes.clone()));
			}
			list.add(&PlutusData::new_list(&candidate_datum));
		}
		let appendix = PlutusData::new_list(&list);
		VersionedGenericDatum {
			datum: PlutusData::new_empty_constr_plutus_data(&BigNum::zero()),
			appendix,
			version: 0,
		}
		.into()
	}

	fn candidates_to_plutus_data_v1(candidates: &[PermissionedCandidateData]) -> PlutusData {
		let mut list = PlutusList::new();
		for candidate in candidates {
			let mut candidate_datum = PlutusList::new();
			candidate_datum.add(&PlutusData::new_bytes(candidate.sidechain_public_key.0.clone()));
			candidate_datum.add(&candidate_keys_to_plutus(&candidate.keys));
			list.add(&PlutusData::new_list(&candidate_datum));
		}
		VersionedGenericDatum {
			datum: PlutusData::new_empty_constr_plutus_data(&BigNum::zero()),
			appendix: PlutusData::new_list(&list),
			version: 1,
		}
		.into()
	}

	if candidates.iter().all(|c| c.keys.has_only_aura_and_grandpa_keys()) {
		candidates_to_plutus_data_v0(candidates)
	} else {
		candidates_to_plutus_data_v1(candidates)
	}
}

impl PermissionedCandidateDatums {
	/// Parses plutus data schema in accordance with V1 schema
	fn decode_v1_appendix(data: &PlutusData) -> Result<Self, String> {
		let permissioned_candidates = data
			.as_list()
			.and_then(|list_datums| {
				list_datums
					.into_iter()
					.map(decode_v1_candidate_datum)
					.collect::<Option<Vec<PermissionedCandidateDatumV1>>>()
			})
			.ok_or("Expected [[ByteString, ByteString], [[ByteString, ByteString], ... ]]")?;
		Ok(Self::V1(permissioned_candidates))
	}
}

impl VersionedDatumWithLegacy for PermissionedCandidateDatums {
	const NAME: &str = "PermissionedCandidateDatums";

	/// Parses plutus data schema that was used before datum versioning was added. Kept for backwards compatibility.
	fn decode_legacy(data: &PlutusData) -> Result<Self, String> {
		let permissioned_candidates = data
			.as_list()
			.and_then(|list_datums| {
				list_datums
					.into_iter()
					.map(decode_legacy_candidate_datum)
					.collect::<Option<Vec<PermissionedCandidateDatumV0>>>()
			})
			.ok_or("Expected [[ByteString, ByteString, ByteString]]")?;

		Ok(Self::V0(permissioned_candidates))
	}

	fn decode_versioned(
		version: u64,
		_datum: &PlutusData,
		appendix: &PlutusData,
	) -> Result<Self, String> {
		match version {
			// v0 appendix is the same as legacy format of whole plutus data
			0 => PermissionedCandidateDatums::decode_legacy(appendix)
				.map_err(|msg| format!("Cannot parse appendix: {msg}")),
			1 => PermissionedCandidateDatums::decode_v1_appendix(appendix)
				.map_err(|msg| format!("Cannot parse appendix: {msg}")),
			_ => Err(format!("Unknown version: {version}")),
		}
	}
}

/// Decodes whatever looks syntactically correct, leaving validation for runtime.
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

/// Decodes whatever looks syntactically correct, leaving validation for runtime.
fn decode_v1_candidate_datum(datum: &PlutusData) -> Option<PermissionedCandidateDatumV1> {
	// The first element has Partner Chains key, second contains all other keys
	let outer_list = datum.as_list().filter(|l| l.len() == 2)?;
	let partner_chains_key = SidechainPublicKey(outer_list.get(0).as_bytes()?);
	let keys = decode_candidate_keys(&outer_list.get(1))?;
	Some(PermissionedCandidateDatumV1 { partner_chains_key, keys })
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;
	use sp_core::crypto::key_types::{AURA, GRANDPA};

	#[test]
	fn valid_legacy_permissioned_candidates() {
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

	fn v0_datum_json() -> serde_json::Value {
		serde_json::json!({
			"list": [
				{ "constructor": 0, "fields": [] },
				{ "list": [
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
				]},
				{ "int": 0 }
			]
		})
	}

	fn v1_datum_json() -> serde_json::Value {
		serde_json::json!({
			"list": [
				{ "constructor": 0, "fields": [] },
				{"list": [
					{"list":[
						{"bytes": "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"},
						{"list": [
							{"list": [{"bytes": hex::encode(b"aura")}, {"bytes": "bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec"}]},
							{"list": [{"bytes": hex::encode(b"gran")}, {"bytes": "9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d"}]}
						]}
					]},
					{"list":[
						{"bytes": "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"},
						{"list": [
							{"list": [{"bytes": hex::encode(b"aura")}, {"bytes": "56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19"}]},
							{"list": [{"bytes": hex::encode(b"gran")}, {"bytes": "7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32"}]}
						]}
					]}
				]},
				{ "int": 1 }
			]
		})
	}

	#[test]
	fn permissioned_candidates_to_plutus_data_outputs_v0_for_aura_and_grandpa_keys() {
		let expected_plutus_data = json_to_plutus_data(v0_datum_json());

		let domain_data = vec![
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(
					hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854")
						.to_vec(),
				),
				keys: CandidateKeys(vec![
					AuraPublicKey(
						hex!("bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec")
							.to_vec(),
					)
					.into(),
					GrandpaPublicKey(
						hex!("9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d")
							.to_vec(),
					)
					.into(),
				]),
			},
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(
					hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf")
						.to_vec(),
				),
				keys: CandidateKeys(vec![
					AuraPublicKey(
						hex!("56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19")
							.to_vec(),
					)
					.into(),
					GrandpaPublicKey(
						hex!("7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32")
							.to_vec(),
					)
					.into(),
				]),
			},
		];
		assert_eq!(permissioned_candidates_to_plutus_data(&domain_data), expected_plutus_data)
	}

	#[test]
	fn permissioned_candidates_to_plutus_data_outputs_v1() {
		let domain_data = vec![
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey([1; 33].to_vec()),
				keys: CandidateKeys(vec![
					CandidateKey { id: [2; 4], bytes: [3; 32].to_vec() },
					CandidateKey { id: [4; 4], bytes: [5; 32].to_vec() },
				]),
			},
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey([6; 33].to_vec()),
				keys: CandidateKeys(vec![
					CandidateKey { id: [7; 4], bytes: [8; 32].to_vec() },
					CandidateKey { id: [9; 4], bytes: [10u8; 32].to_vec() },
				]),
			},
		];
		let json = serde_json::json!({
			"list": [
				{ "constructor": 0, "fields": [] },
				{"list": [
					{"list":[
						{"bytes": "010101010101010101010101010101010101010101010101010101010101010101"},
						{"list": [
							{"list": [{"bytes": "02020202"}, {"bytes": "0303030303030303030303030303030303030303030303030303030303030303"}]},
							{"list": [{"bytes": "04040404"}, {"bytes": "0505050505050505050505050505050505050505050505050505050505050505"}]}
						]}
					]},
					{"list":[
						{"bytes": "060606060606060606060606060606060606060606060606060606060606060606"},
						{"list": [
							{"list": [{"bytes": "07070707"}, {"bytes": "0808080808080808080808080808080808080808080808080808080808080808"}]},
							{"list": [{"bytes": "09090909"}, {"bytes": "0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a"}]}
						]}
					]}
				]},
				{ "int": 1 }
			]
		});
		let expected_plutus_data = json_to_plutus_data(json);
		assert_eq!(permissioned_candidates_to_plutus_data(&domain_data), expected_plutus_data)
	}

	#[test]
	fn valid_v0_permissioned_candidates() {
		let plutus_data = json_to_plutus_data(v0_datum_json());

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

	#[test]
	fn valid_v1_permissioned_candidates() {
		let plutus_data = json_to_plutus_data(v1_datum_json());

		let expected_datum = PermissionedCandidateDatums::V1(vec![
			PermissionedCandidateDatumV1 {
				partner_chains_key: SidechainPublicKey(
					hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854").into(),
				),
				keys: CandidateKeys(vec![
					CandidateKey::new(
						AURA,
						hex!("bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec")
							.into(),
					),
					CandidateKey::new(
						GRANDPA,
						hex!("9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d")
							.into(),
					),
				]),
			},
			PermissionedCandidateDatumV1 {
				partner_chains_key: SidechainPublicKey(
					hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf").into(),
				),
				keys: CandidateKeys(vec![
					CandidateKey::new(
						AURA,
						hex!("56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19")
							.into(),
					),
					CandidateKey::new(
						GRANDPA,
						hex!("7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32")
							.into(),
					),
				]),
			},
		]);

		assert_eq!(PermissionedCandidateDatums::try_from(plutus_data).unwrap(), expected_datum)
	}
}
