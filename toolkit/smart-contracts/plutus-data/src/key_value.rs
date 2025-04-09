use crate::{DataDecodingError, DecodingResult};
use cardano_serialization_lib::{PlutusData, PlutusList};
use sidechain_domain::byte_string::ByteString;

#[derive(Clone, Debug, PartialEq)]
pub struct KeyValueDatum {
	pub key: String,
	pub value: ByteString,
}

impl TryFrom<PlutusData> for KeyValueDatum {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		let data_iter = datum.as_list().ok_or_else(|| DataDecodingError {
			datum: datum.clone(),
			to: "KeyValueDatum".into(),
			msg: "Expected [String, Vec<u8>]".into(),
		})?;
		let key = data_iter.get(0).as_bytes().ok_or_else(|| DataDecodingError {
			datum: datum.clone(),
			to: "KeyValueDatum".into(),
			msg: "Expected [String, Vec<u8>]".into(),
		})?;
		let value = data_iter.get(1).as_bytes().ok_or_else(|| DataDecodingError {
			datum: datum.clone(),
			to: "KeyValueDatum".into(),
			msg: "Expected [String, Vec<u8>]".into(),
		})?;
		Ok(Self { key: String::from_utf8(key).unwrap(), value: value.into() })
	}
}

impl From<KeyValueDatum> for sidechain_domain::KeyValue {
	fn from(datum: KeyValueDatum) -> Self {
		Self { key: datum.key, value: datum.value }
	}
}

pub fn key_value_to_plutus_data(key_value: &sidechain_domain::KeyValue) -> PlutusData {
	let mut list = PlutusList::new();
	list.add(&PlutusData::new_bytes(key_value.key.as_bytes().to_vec()));
	list.add(&PlutusData::new_bytes(key_value.value.0.clone()));
	PlutusData::new_list(&list)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use pretty_assertions::assert_eq;

	#[test]
	fn valid_key_value() {
		let plutus_data = test_plutus_data!({"list": [{"bytes": "6b6579"}, {"bytes": "ddeeff"}]});

		let expected_datum =
			KeyValueDatum { key: "key".to_string(), value: hex::decode("ddeeff").unwrap().into() };

		assert_eq!(KeyValueDatum::try_from(plutus_data).unwrap(), expected_datum)
	}

	#[test]
	fn domain_key_value_to_csl() {
		let key_value = sidechain_domain::KeyValue {
			key: "key".to_string(),
			value: hex::decode("ddeeff").unwrap().into(),
		};

		let expected_plutus_data = json_to_plutus_data(datum_json());

		assert_eq!(key_value_to_plutus_data(&key_value), expected_plutus_data)
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
