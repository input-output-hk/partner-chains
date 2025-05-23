//! Common types used in the Ogmios API.

use serde::{Deserialize, Deserializer};
use sidechain_domain::{McTxHash, UtxoId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Default)]
/// Represents the length of a slot in milliseconds.
pub struct SlotLength {
	/// The length of a slot in milliseconds.
	pub milliseconds: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
/// Represents the time in seconds.
pub struct TimeSeconds {
	/// The time in seconds.
	pub seconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Default)]
/// Represents the size of a transaction in bytes.
pub struct OgmiosBytesSize {
	/// The size of a transaction in bytes.
	pub bytes: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Represents a UTXO.
pub struct OgmiosUtxo {
	/// The transaction hash.
	pub transaction: OgmiosTx,
	/// The index of the UTXO within the transaction.
	pub index: u16,
	/// The Bech32 address of the UTXO.
	pub address: String,
	/// The value of the UTXO.
	pub value: OgmiosValue,
	/// The datum of the UTXO.
	pub datum: Option<Datum>,
	/// The hash of the datum of the UTXO.
	pub datum_hash: Option<DatumHash>,
	/// The reference script of the UTXO.
	pub script: Option<OgmiosScript>,
}

impl OgmiosUtxo {
	/// Returns the UTXO ID.
	pub fn utxo_id(&self) -> UtxoId {
		UtxoId::new(self.transaction.id, self.index)
	}
}

impl core::fmt::Display for OgmiosUtxo {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}#{}", hex::encode(self.transaction.id), self.index)
	}
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
/// Represents a datum.
pub struct Datum {
	/// The bytes of the datum.
	#[serde(deserialize_with = "parse_bytes")]
	pub bytes: Vec<u8>,
}

impl From<Vec<u8>> for Datum {
	fn from(bytes: Vec<u8>) -> Self {
		Datum { bytes }
	}
}

impl std::fmt::Debug for Datum {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Datum").field("bytes", &hex::encode(&self.bytes)).finish()
	}
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
/// Represents a datum hash.
pub struct DatumHash {
	/// The bytes of the datum hash.
	#[serde(deserialize_with = "parse_bytes_array")]
	pub bytes: [u8; 32],
}

impl From<[u8; 32]> for DatumHash {
	fn from(bytes: [u8; 32]) -> Self {
		DatumHash { bytes }
	}
}

impl std::fmt::Debug for DatumHash {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("DatumHash").field("bytes", &hex::encode(self.bytes)).finish()
	}
}

#[derive(Clone, Deserialize, Eq, PartialEq)]
/// Represents a cardano script.
pub struct OgmiosScript {
	/// The language of the script.
	pub language: String,
	/// The CBOR representation of the script (in case of Plutus scripts).
	#[serde(deserialize_with = "parse_bytes")]
	pub cbor: Vec<u8>,
	/// The JSON representation of the script (in case of Native scripts).
	pub json: Option<NativeScript>,
}

impl std::fmt::Debug for OgmiosScript {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("PlutusScript")
			.field("language", &self.language)
			.field("cbor", &hex::encode(&self.cbor))
			.field("json", &self.json)
			.finish()
	}
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "clause", rename_all = "lowercase")]
/// Represents a cardano native script.
pub enum NativeScript {
	/// Represents a signature script.
	Signature {
		#[serde(deserialize_with = "parse_bytes_array")]
		/// The public key hash of the signer.
		from: [u8; 28],
	},
	/// Represents an all script.
	All {
		/// The scripts to check.
		from: Vec<NativeScript>,
	},
	/// Represents an any script.
	Any {
		/// The scripts to check.
		from: Vec<NativeScript>,
	},
	#[serde(rename_all = "camelCase")]
	/// Represents a some script.
	Some {
		/// The scripts to check.
		from: Vec<NativeScript>,
		/// The minimum number of scripts that must be satisfied.
		at_least: u32,
	},
	/// Represents a before script.
	Before {
		/// The slot number.
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

/// Represents a script hash.
type ScriptHash = [u8; 28];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
/// Represents the value of a UTXO.
pub struct OgmiosValue {
	/// The amount of lovelace in the UTXO.
	pub lovelace: u64,
	/// The native tokens in the UTXO.
	pub native_tokens: HashMap<ScriptHash, Vec<Asset>>,
}

impl OgmiosValue {
	/// Creates a new UTXO value with only lovelace.
	pub fn new_lovelace(lovelace: u64) -> Self {
		Self { lovelace, native_tokens: HashMap::new() }
	}
}

/// Represents an asset of an UTXO.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
	/// The name of the asset.
	pub name: Vec<u8>,
	/// The amount of the asset.
	pub amount: u64,
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
							.and_then(|n| n.clone().as_u64())
							.ok_or("expected asset amount to be u64");
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

#[derive(Clone, Default, Deserialize, Eq, PartialEq)]
/// Transaction identifier.
pub struct OgmiosTx {
	/// The transaction hash.
	#[serde(deserialize_with = "parse_bytes_array")]
	pub id: [u8; 32],
}

impl From<McTxHash> for OgmiosTx {
	fn from(id: McTxHash) -> Self {
		Self { id: id.0 }
	}
}

impl From<[u8; 32]> for OgmiosTx {
	fn from(id: [u8; 32]) -> Self {
		Self { id }
	}
}

impl Debug for OgmiosTx {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("OgmiosTx").field("id", &hex::encode(self.id)).finish()
	}
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
	use crate::types::{Asset, NativeScript, OgmiosScript, OgmiosTx, OgmiosValue};
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
			vec![Asset { name: vec![], amount: 18446744073709551615u64 }]
		);
		let assets = value
			.native_tokens
			.get(&hex!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
			.unwrap()
			.clone();
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
				script: Some(OgmiosScript {
					language: "plutus:v3".into(),
					cbor: hex!("aabbccdd00112233").to_vec(),
					json: None,
				})
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
			"script": {
				"language": "native",
				"json": {
					"clause": "some",
					"atLeast": 1,
					"from":[
						{"clause": "signature","from": "a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7"},
						{"clause": "before", "slot": 100 }
					]
				},
				"cbor": "830301818200581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
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
				script: Some(OgmiosScript {
					language: "native".into(),
					json: Some(NativeScript::Some {
						from: vec![
							NativeScript::Signature {
								from: hex!(
									"a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7"
								)
							},
							NativeScript::Before { slot: 100 }
						],
						at_least: 1
					}),
					cbor: hex!(
						"830301818200581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
					)
					.to_vec()
				})
			}
		)
	}
}
