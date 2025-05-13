//! Plutus data types for governed map.
use crate::{DataDecodingError, DecodingResult, decoding_error_and_log};
use cardano_serialization_lib::{PlutusData, PlutusList};
use sidechain_domain::byte_string::ByteString;

#[derive(Clone, Debug, PartialEq)]
/// Datum for governed map entries
pub struct GovernedMapDatum {
	/// Governed map entry key.
	pub key: String,
	/// Governed map entry value.
	pub value: ByteString,
}

impl GovernedMapDatum {
	/// Constructor for [GovernedMapDatum]
	pub fn new(key: String, value: ByteString) -> Self {
		Self { key, value }
	}
}

impl TryFrom<PlutusData> for GovernedMapDatum {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		let error = || {
			decoding_error_and_log(&datum, "GovernedMapDatum", "Expected List([UTF8String, Bytes])")
		};

		datum
			.as_list()
			.and_then(|list| {
				list.get(0)
					.as_bytes()
					.and_then(|key| String::from_utf8(key).ok().map(|key| (list, key)))
			})
			.and_then(|(list, key)| list.get(1).as_bytes().map(|value| (key, value)))
			.ok_or_else(error)
			.map(|(key, value)| Self { key, value: value.into() })
	}
}

/// Converts [GovernedMapDatum] to [PlutusData].
pub fn governed_map_datum_to_plutus_data(governed_map_datum: &GovernedMapDatum) -> PlutusData {
	let mut list = PlutusList::new();
	list.add(&PlutusData::new_bytes(governed_map_datum.key.as_bytes().to_vec()));
	list.add(&PlutusData::new_bytes(governed_map_datum.value.0.clone()));
	PlutusData::new_list(&list)
}

impl From<GovernedMapDatum> for PlutusData {
	fn from(governed_map_datum: GovernedMapDatum) -> Self {
		governed_map_datum_to_plutus_data(&governed_map_datum)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;

	#[test]
	fn governed_map_from_plutus_data() {
		let plutus_data = test_plutus_data!({"list": [{"bytes": "6b6579"}, {"bytes": "ddeeff"}]});

		let expected_datum =
			GovernedMapDatum { key: "key".to_string(), value: hex!("ddeeff").to_vec().into() };

		assert_eq!(GovernedMapDatum::try_from(plutus_data).unwrap(), expected_datum)
	}

	#[test]
	fn governed_map_to_plutus_data() {
		let governed_map_datum =
			GovernedMapDatum { key: "key".to_string(), value: hex!("ddeeff").to_vec().into() };

		let expected_plutus_data = json_to_plutus_data(datum_json());

		assert_eq!(governed_map_datum_to_plutus_data(&governed_map_datum), expected_plutus_data)
	}

	fn datum_json() -> serde_json::Value {
		serde_json::json!({
			"list": [
				{ "bytes": "6b6579" },
				{ "bytes": "ddeeff" },
			]
		})
	}
}
