//! Common types used in the Ogmios API.

use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SlotLength {
	pub milliseconds: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TimeSeconds {
	pub seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct OgmiosBytesSize {
	pub bytes: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OgmiosUtxo {
	pub transaction: OgmiosTx,
	pub index: u32,
	// bech32 address
	pub address: String,
	pub value: OgmiosValue,
	pub datum: Option<Datum>,
	pub datum_hash: Option<DatumHash>,
	pub script: Option<OgmiosScript>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct Datum {
	#[serde(deserialize_with = "parse_bytes")]
	pub bytes: Vec<u8>,
}

impl From<Vec<u8>> for Datum {
	fn from(bytes: Vec<u8>) -> Self {
		Datum { bytes }
	}
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct DatumHash {
	#[serde(deserialize_with = "parse_bytes_array")]
	pub bytes: [u8; 32],
}

impl From<[u8; 32]> for DatumHash {
	fn from(bytes: [u8; 32]) -> Self {
		DatumHash { bytes }
	}
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum OgmiosScript {
	Plutus(PlutusScript),
	Native(NativeScript),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PlutusScript {
	pub language: String,
	#[serde(deserialize_with = "parse_bytes")]
	pub cbor: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "clause", rename_all = "lowercase")]
pub enum NativeScript {
	Signature {
		#[serde(deserialize_with = "parse_bytes_array")]
		from: [u8; 28],
	},
	All {
		from: Vec<NativeScript>,
	},
	Any {
		from: Vec<NativeScript>,
	},
	#[serde(rename_all = "camelCase")]
	Some {
		from: Vec<NativeScript>,
		at_least: u32,
	},
	Before {
		slot: u64,
	},
}

impl<'de> Deserialize<'de> for OgmiosValue {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value = serde_json::Value::deserialize(deserializer)?;
		TryFrom::try_from(value)
			.map_err(|e| serde::de::Error::custom(format!("failed to parse OgmiosValue: {e}")))
	}
}

type ScriptHash = [u8; 28];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OgmiosValue {
	pub lovelace: u64,
	pub native_tokens: HashMap<ScriptHash, Vec<Asset>>,
}

impl OgmiosValue {
	pub fn new_lovelace(lovelace: u64) -> Self {
		Self { lovelace, native_tokens: HashMap::new() }
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
	pub name: Vec<u8>,
	pub amount: i128,
}

impl TryFrom<serde_json::Value> for OgmiosValue {
	type Error = &'static str;
	fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
		let value = value.as_object().ok_or("expected top level object")?;
		let mut lovelace = 0u64;
		let mut native_tokens = HashMap::new();
		value.into_iter().try_for_each(|(policy_id, assets)| {
			let asset_to_amount = assets.as_object().ok_or("expected an object")?;
			if policy_id == "ada" {
				let amount = asset_to_amount.get("lovelace").ok_or("expected lovelace amount")?;
				lovelace = amount.as_u64().ok_or("expected lovelace amount to be u64")?;
				Ok::<(), &'static str>(())
			} else {
				let policy_id = hex::decode(policy_id)
					.map_err(|_| "expected policy id to be hexstring")?
					.try_into()
					.map_err(|_| "expected policy id to be 28 bytes")?;
				let assets: Result<Vec<_>, &str> = asset_to_amount
					.into_iter()
					.map(|(asset_name, amount)| {
						let name = hex::decode(asset_name)
							.map_err(|_| "expected asset name to be hexstring");
						let amount = amount
							.as_number()
							.and_then(|n| n.clone().as_i128())
							.ok_or("expected amount to be i128");
						name.and_then(|name| amount.map(|amount| Asset { name, amount }))
					})
					.collect();
				native_tokens.insert(policy_id, assets?);
				Ok::<(), &'static str>(())
			}
		})?;
		Ok(Self { lovelace, native_tokens })
	}
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct OgmiosTx {
	#[serde(deserialize_with = "parse_bytes_array")]
	pub id: [u8; 32],
}

pub(crate) fn parse_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let buf = String::deserialize(deserializer)?;
	hex::decode(buf).map_err(serde::de::Error::custom)
}

pub(crate) fn parse_bytes_array<'de, D, const N: usize>(
	deserializer: D,
) -> Result<[u8; N], D::Error>
where
	D: Deserializer<'de>,
{
	let bytes = parse_bytes(deserializer)?;
	TryFrom::try_from(bytes).map_err(|_| serde::de::Error::custom(format!("expected {} bytes", N)))
}

pub(crate) fn parse_fraction_decimal<'de, D>(deserializer: D) -> Result<fraction::Decimal, D::Error>
where
	D: Deserializer<'de>,
{
	let buf = String::deserialize(deserializer)?;
	fraction::Decimal::from_str(&buf).map_err(serde::de::Error::custom)
}

pub(crate) fn parse_fraction_ratio_u64<'de, D>(
	deserializer: D,
) -> Result<fraction::Ratio<u64>, D::Error>
where
	D: Deserializer<'de>,
{
	let buf = String::deserialize(deserializer)?;
	fraction::Ratio::<u64>::from_str(&buf).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
	use super::OgmiosUtxo;
	use crate::types::{Asset, NativeScript, OgmiosScript, OgmiosTx, OgmiosValue, PlutusScript};
	use hex_literal::hex;

	#[test]
	fn parse_ada_only_value() {
		let value = serde_json::json!({
			"ada": {
				"lovelace": 18446744073709551615u64
			}
		});
		let value: OgmiosValue = serde_json::from_value(value).unwrap();
		assert_eq!(value.lovelace, 18446744073709551615u64);
		assert_eq!(value.native_tokens.len(), 0);
	}

	#[test]
	fn parse_value_with_native_tokens() {
		let value = serde_json::json!({
			"ada": {
				"lovelace": 3
			},
			"e0d4479b3dbb53b1aecd48f7ef524a9cf166585923d91d9c72ed02cb": {
				"": 18446744073709551615i128
			},
			"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa": {
				"cdef": -18446744073709551615i128,
				"aaaa": 1,
			}
		});
		let value: OgmiosValue = serde_json::from_value(value).unwrap();
		assert_eq!(value.lovelace, 3);
		assert_eq!(
			value
				.native_tokens
				.get(&hex!("e0d4479b3dbb53b1aecd48f7ef524a9cf166585923d91d9c72ed02cb"))
				.unwrap()
				.clone(),
			vec![Asset { name: vec![], amount: 18446744073709551615i128 }]
		);
		let assets = value
			.native_tokens
			.get(&hex!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
			.unwrap()
			.clone();
		assert_eq!(
			assets.iter().find(|asset| asset.name == hex!("cdef").to_vec()).unwrap().amount,
			-18446744073709551615i128
		);
		assert_eq!(
			assets.iter().find(|asset| asset.name == hex!("aaaa").to_vec()).unwrap().amount,
			1
		);
	}

	#[test]
	fn parse_utxo_with_datum() {
		let value = serde_json::json!({
			"transaction": { "id": "106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580" },
			"index": 1,
			"address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
			"value": { "ada": {	"lovelace": 1356118 } },
			"datum": "d8799fff",
			"datumHash": "c248757d390181c517a5beadc9c3fe64bf821d3e889a963fc717003ec248757d"
		});
		let utxo: OgmiosUtxo = serde_json::from_value(value).unwrap();
		assert_eq!(
			utxo,
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580")
				},
				index: 1,
				address: "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy"
					.to_string(),
				value: OgmiosValue::new_lovelace(1356118),
				datum: Some(hex!("d8799fff").to_vec().into()),
				datum_hash: Some(
					hex!("c248757d390181c517a5beadc9c3fe64bf821d3e889a963fc717003ec248757d").into()
				),
				script: None,
			}
		)
	}

	#[test]
	fn parse_utxo_with_plutus_script() {
		let value = serde_json::json!({
			"transaction": {
			  "id": "106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580"
			},
			"index": 1,
			"address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
			"value": { "ada": { "lovelace": 1356118 } },
			"script": {
				"cbor": "aabbccdd00112233",
				"language": "plutus:v3"
			}
		});
		let utxo: OgmiosUtxo = serde_json::from_value(value).unwrap();
		assert_eq!(
			utxo,
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580")
				},
				index: 1,
				address: "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy"
					.to_string(),
				value: OgmiosValue::new_lovelace(1356118),
				datum: None,
				datum_hash: None,
				script: Some(OgmiosScript::Plutus(PlutusScript {
					language: "plutus:v3".into(),
					cbor: hex!("aabbccdd00112233").to_vec()
				}))
			}
		)
	}

	#[test]
	fn parse_utxo_with_native_script() {
		let value = serde_json::json!({
			"transaction": { "id": "106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580" },
			"index": 1,
			"address": "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
			"value": { "ada": {	"lovelace": 1356118 } },
			"script": {"clause": "some", "atLeast": 1, "from":[{"clause": "signature","from": "a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7"}, {"clause": "before", "slot": 100 }]}
		});
		let utxo: OgmiosUtxo = serde_json::from_value(value).unwrap();
		assert_eq!(
			utxo,
			OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("106b0d7d1544c97941777041699412fb7c8b94855210987327199620c0599580")
				},
				index: 1,
				address: "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy"
					.to_string(),
				value: OgmiosValue::new_lovelace(1356118),
				datum: None,
				datum_hash: None,
				script: Some(OgmiosScript::Native(NativeScript::Some {
					from: vec![
						NativeScript::Signature {
							from: hex!("a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7")
						},
						NativeScript::Before { slot: 100 }
					],
					at_least: 1
				}))
			}
		)
	}
}
