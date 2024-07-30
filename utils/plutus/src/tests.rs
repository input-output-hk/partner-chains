use crate::Datum::{ByteStringDatum, ConstructorDatum, IntegerDatum, ListDatum, MapDatum};
use crate::ToDatum;
use crate::{Datum, FromJsonError, MapDatumEntry};
use alloc::vec;
use num_bigint::BigInt;
use serde_json::json;

#[test]
pub fn from_integer_datum() {
	let datum = Datum::try_from(&json!({"int": 77777})).unwrap();
	assert_eq!(IntegerDatum(BigInt::from(77777)), datum);
}

#[test]
pub fn from_integer_invalid() {
	let result = Datum::try_from(&json!({"int": "77777"}));
	assert_eq!(Err(FromJsonError::ExpectedIntegerDatumValue), result);
}

#[test]
pub fn from_bytes_valid() {
	let datum = Datum::try_from(&json!({"bytes": "010101010101"})).unwrap();
	assert_eq!(ByteStringDatum([1u8; 6].to_vec()), datum);
}

#[test]
pub fn from_bytes_invalid_not_string() {
	let result = Datum::try_from(&json!({"bytes": 9328723248u64}));
	assert_eq!(Err(FromJsonError::ExpectedHexEncodedBytes), result);
}

#[test]
pub fn from_bytes_invalid_not_hex() {
	let result = Datum::try_from(&json!({"bytes": "thisisnothex"}));
	assert_eq!(Err(FromJsonError::ExpectedHexEncodedBytes), result);
}

#[test]
pub fn from_list() {
	let datum = Datum::try_from(&json!({"list": [{"int": 77777}, {"int": 88888}]})).unwrap();
	assert_eq!(ListDatum(vec![77777u32.to_datum(), 88888u32.to_datum()]), datum);
}

#[test]
pub fn from_list_invalid() {
	let result = Datum::try_from(&json!({"list": "abcd"}));
	assert_eq!(Err(FromJsonError::ExpectedArrayOfDatums), result);
}

#[test]
pub fn from_constructor_valid() {
	let json = json!({"constructor": 17, "fields":[{"int": 77777}, {"int": 88888}]});
	let datum = Datum::try_from(&json).unwrap();
	assert_eq!(
		ConstructorDatum {
			constructor: 17,
			fields: vec![77777u32.to_datum(), 88888u32.to_datum()]
		},
		datum
	);
}

#[test]
pub fn from_constructor_valid_empty() {
	let datum = Datum::try_from(&json!({"constructor": 100, "fields":[]})).unwrap();
	assert_eq!(ConstructorDatum { constructor: 100, fields: vec![] }, datum);
}

#[test]
pub fn from_map_valid() {
	let json = json!({"map": [
		{"key": {"int": 77777}, "value": {"bytes": "010203"}},
		{"key": {"bytes": "aaaaaa"}, "value": {"int": 444}},
	]});
	let datum = Datum::try_from(&json).unwrap();
	assert_eq!(
		MapDatum(vec![
			MapDatumEntry {
				key: 77777u32.to_datum(),
				value: ByteStringDatum([1u8, 2u8, 3u8].to_vec())
			},
			MapDatumEntry {
				key: ByteStringDatum([170u8, 170u8, 170u8].to_vec()),
				value: 444u32.to_datum(),
			},
		]),
		datum
	);
}

#[test]
pub fn integer_datum_to_json() {
	let datum = 77777u32.to_datum();
	let json_value = serde_json::Value::from(&datum);
	assert_eq!(json_value, json!({	"int": 77777 }));
}

#[test]
pub fn bytestring_datum_to_json() {
	let datum = ByteStringDatum([1u8; 6].to_vec());
	let json_value = serde_json::Value::from(&datum);
	assert_eq!(json_value, json!({"bytes": "010101010101"}));
}

#[test]
pub fn list_datum_to_json() {
	let datum = ListDatum(vec![77777u32.to_datum(), 88888u32.to_datum()]);
	let json_value = serde_json::Value::from(&datum);
	assert_eq!(json_value, json!({"list": [{"int": 77777}, {"int": 88888}]}));
}

#[test]
pub fn constructor_datum_to_json() {
	let datum = ConstructorDatum { constructor: 7, fields: vec![13u32.to_datum()] };
	let json_value = serde_json::Value::from(&datum);
	assert_eq!(json_value, json!({"constructor": 7, "fields": [{"int": 13}]}));
}
