//! Plutus data types for permissioned candidates.
use cardano_serialization_lib::{BigNum, PlutusData, PlutusList};
use sidechain_domain::*;

use crate::{
	DataDecodingError, DecodingResult, VersionedDatum, VersionedDatumWithLegacy,
	VersionedGenericDatum,
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

impl TryFrom<PlutusData> for PermissionedCandidateDatums {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		Self::decode(&datum)
	}
}

impl From<PermissionedCandidateDatumV0> for PermissionedCandidateData {
	fn from(value: PermissionedCandidateDatumV0) -> Self {
		Self::V0(PermissionedCandidateDataV0 {
			sidechain_public_key: value.sidechain_public_key,
			aura_public_key: value.aura_public_key,
			grandpa_public_key: value.grandpa_public_key,
		})
	}
}

impl From<PermissionedCandidateDatumV1> for PermissionedCandidateData {
	fn from(value: PermissionedCandidateDatumV1) -> Self {
		Self::V1(PermissionedCandidateDataV1 { keys: value.keys })
	}
}

#[derive(Clone, Debug, PartialEq)]
/// Datum representing a premissioned candidate with arbitraty set of keys
pub struct PermissionedCandidateDatumV1 {
	/// Represents arbitrary set of keys with 4 character identifier
	pub keys: Vec<([u8; 4], Vec<u8>)>,
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
///
/// Encoding:
///   VersionedGenericDatum:
///   - datum: ()
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
pub fn permissioned_candidates_to_plutus_data(
	candidates: &[PermissionedCandidateData],
) -> PlutusData {
	let mut list = PlutusList::new();
	for candidate in candidates {
		let mut candidate_datum = PlutusList::new();
		match candidate {
			PermissionedCandidateData::V0(data) => {
				candidate_datum.add(&PlutusData::new_bytes(data.sidechain_public_key.0.clone()));
				candidate_datum.add(&PlutusData::new_bytes(data.aura_public_key.0.clone()));
				candidate_datum.add(&PlutusData::new_bytes(data.grandpa_public_key.0.clone()));
			},
			PermissionedCandidateData::V1(data) => {
				for (key_type, key) in &data.keys {
					let mut key_type_with_key = PlutusList::new();
					key_type_with_key.add(&PlutusData::new_bytes(key_type.to_vec()));
					key_type_with_key.add(&PlutusData::new_bytes(key.to_vec()));
					candidate_datum.add(&PlutusData::new_list(&key_type_with_key));
				}
			},
		};
		list.add(&PlutusData::new_list(&candidate_datum));
	}
	let appendix = PlutusData::new_list(&list);
	VersionedGenericDatum {
		datum: PlutusData::new_empty_constr_plutus_data(&BigNum::zero()),
		appendix,
		version: 1,
	}
	.into()
}

impl PermissionedCandidateDatums {
	/// Parses plutus data schema in accordance with V1 schema
	fn decode_v1(data: &PlutusData) -> Result<Self, String> {
		let permissioned_candidates = data
			.as_list()
			.and_then(|list_datums| {
				list_datums
					.into_iter()
					.map(decode_v1_candidate_datum)
					.collect::<Option<Vec<PermissionedCandidateDatumV1>>>()
			})
			.ok_or("Expected [[ByteString, ByteString], [ByteString,ByteString], ... ]")?;
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
			0 => PermissionedCandidateDatums::decode_legacy(appendix)
				.map_err(|msg| format!("Cannot parse appendix: {msg}")),
			1 => PermissionedCandidateDatums::decode_v1(appendix)
				.map_err(|msg| format!("Cannot parse appendix: {msg}")),
			_ => Err(format!("Unknown version: {version}")),
		}
	}
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

fn decode_v1_candidate_datum(datum: &PlutusData) -> Option<PermissionedCandidateDatumV1> {
	let keys = datum
		.as_list()
		.iter()
		.map(|datum| {
			(datum.get(0).as_bytes().unwrap().try_into().unwrap(), datum.get(1).as_bytes().unwrap())
		})
		.collect();

	Some(PermissionedCandidateDatumV1 { keys })
}

#[cfg(test)]
mod plutus_data {
	use super::*;
	use crate::test_helpers::*;
	use hex_literal::hex;

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
				{ "int": 1 }
			]
		})
	}

	fn v1_datum_json() -> serde_json::Value {
		serde_json::json!({
			"list": [
				{ "constructor": 0, "fields": [] },
				{"list": [
					{"list": [
						{"bytes": hex::encode(b"crch")},
						{"bytes": "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"}
					]},
					{"list": [
						{"bytes": hex::encode(b"aura")},
						{"bytes": "bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec"}
					]},
					{"list": [
						{"bytes": hex::encode(b"gran")},
						{"bytes": "9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d"}
					]}
				]},
				{"list": [
					{"list": [
						{"bytes": hex::encode(b"crch")},
						{"bytes": "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf"}
					]},
					{"list": [
						{"bytes": hex::encode(b"aura")},
						{"bytes": "56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19"}
					]},
					{"list": [
						{"bytes": hex::encode(b"gran")},
						{"bytes": "7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32"}
					]}
				]},
				{ "int": 1 }
			]
		})
	}

	#[test]
	fn test_permissioned_candidates_to_plutus_data() {
		let expected_plutus_data = json_to_plutus_data(v0_datum_json());

		let domain_data = vec![
			PermissionedCandidateData::V0(PermissionedCandidateDataV0 {
				sidechain_public_key: SidechainPublicKey(
					hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854")
						.to_vec(),
				),
				aura_public_key: AuraPublicKey(
					hex!("bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec")
						.to_vec(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d")
						.to_vec(),
				),
			}),
			PermissionedCandidateData::V0(PermissionedCandidateDataV0 {
				sidechain_public_key: SidechainPublicKey(
					hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf")
						.to_vec(),
				),
				aura_public_key: AuraPublicKey(
					hex!("56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19")
						.to_vec(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32")
						.to_vec(),
				),
			}),
		];
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
				keys: vec![
					(
						*b"crch",
						hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854")
							.into(),
					),
					(
						*b"aura",
						hex!("bf20afa1c1a72af3341fa7a447e3f9eada9f3d054a7408fb9e49ad4d6e6559ec")
							.into(),
					),
					(
						*b"gran",
						hex!("9042a40b0b1baa9adcead024432a923eac706be5e1a89d7f2f2d58bfa8f3c26d")
							.into(),
					),
				],
			},
			PermissionedCandidateDatumV1 {
				keys: vec![
					(
						*b"crch",
						hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf")
							.into(),
					),
					(
						*b"aura",
						hex!("56d1da82e56e4cb35b13de25f69a3e9db917f3e13d6f786321f4b0a9dc153b19")
							.into(),
					),
					(
						*b"gran",
						hex!("7392f3ea668aa2be7997d82c07bcfbec3ee4a9a4e01e3216d92b8f0d0a086c32")
							.into(),
					),
				],
			},
		]);

		assert_eq!(PermissionedCandidateDatums::try_from(plutus_data).unwrap(), expected_datum)
	}
}
