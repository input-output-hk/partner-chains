#![no_std]

mod cbor;
#[cfg(test)]
mod tests;

extern crate alloc;

use crate::Datum::*;
use alloc::string::String;
pub use alloc::{vec, vec::Vec};
use core::fmt::{Debug, Formatter};
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use serde_json::{json, Map as JMap, Value as JValue};

#[derive(Clone, PartialEq)]
pub enum Datum {
	IntegerDatum(BigInt),
	ByteStringDatum(Vec<u8>),
	ConstructorDatum { constructor: u64, fields: Vec<Datum> },
	ListDatum(Vec<Datum>),
	MapDatum(Vec<MapDatumEntry>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapDatumEntry {
	key: Datum,
	value: Datum,
}

pub trait ToDatum {
	fn to_datum(&self) -> Datum;
}

impl ToDatum for Vec<u8> {
	fn to_datum(&self) -> Datum {
		ByteStringDatum(self.clone())
	}
}

impl ToDatum for [u8] {
	fn to_datum(&self) -> Datum {
		ByteStringDatum(Vec::from(self))
	}
}

impl<T: ToDatum> ToDatum for Option<T> {
	fn to_datum(&self) -> Datum {
		match self {
			Some(value) => ConstructorDatum { constructor: 0, fields: vec![value.to_datum()] },
			None => ConstructorDatum { constructor: 1, fields: vec![] },
		}
	}
}

impl<T> ToDatum for [T]
where
	T: ToDatum,
{
	fn to_datum(&self) -> Datum {
		ListDatum(self.iter().map(ToDatum::to_datum).collect())
	}
}

impl<T> ToDatum for Vec<T>
where
	T: ToDatum,
{
	fn to_datum(&self) -> Datum {
		ListDatum(self.iter().map(ToDatum::to_datum).collect())
	}
}

impl ToDatum for Datum {
	fn to_datum(&self) -> Datum {
		self.clone()
	}
}

impl Datum {
	fn integer<T>(value: T) -> Datum
	where
		BigInt: From<T>,
	{
		IntegerDatum(BigInt::from(value))
	}

	pub fn as_bytestring(&self) -> Option<&Vec<u8>> {
		match self {
			ByteStringDatum(bytes) => Some(bytes),
			_ => None,
		}
	}
}

macro_rules! impl_int_datum_from_uint {
	($T:ty) => {
		impl ToDatum for $T {
			fn to_datum(&self) -> Datum {
				Datum::integer(*self)
			}
		}
	};
}

impl_int_datum_from_uint!(u16);
impl_int_datum_from_uint!(u32);
impl_int_datum_from_uint!(u64);
impl_int_datum_from_uint!(u128);

impl Debug for Datum {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		match self {
			IntegerDatum(i) => write!(f, "I {}", i),
			ByteStringDatum(bs) => write!(f, "B {}", hex::encode(bs)),
			ConstructorDatum { constructor, fields } => write!(f, "C {} {:?}", constructor, fields),
			ListDatum(ds) => write!(f, "L {:?}", ds),
			MapDatum(map) => {
				write!(f, "M {:?}", map)
			},
		}
	}
}

impl TryFrom<&JValue> for Datum {
	type Error = FromJsonError;

	fn try_from(value: &JValue) -> Result<Self, Self::Error> {
		fn expect_list(value: &JValue) -> Result<Vec<Datum>, FromJsonError> {
			let v: &Vec<JValue> = value.as_array().ok_or(FromJsonError::ExpectedArrayOfDatums)?;
			let list: Vec<Result<Datum, FromJsonError>> = v.iter().map(TryFrom::try_from).collect();
			list.into_iter().collect()
		}

		fn expect_fields(item: Option<(&String, &JValue)>) -> Result<Vec<Datum>, FromJsonError> {
			match item {
				Some((key, value)) if key.as_str() == "fields" => {
					let fields =
						expect_list(value).map_err(|_| FromJsonError::ExpectedFieldsKeyValue)?;
					Ok(fields)
				},
				_ => Err(FromJsonError::ExpectedFieldsKeyValue),
			}
		}

		fn expect_constructor(item: Option<(&String, &JValue)>) -> Result<u64, FromJsonError> {
			match item {
				Some((key, value)) if key.as_str() == "constructor" => {
					value.as_u64().ok_or(FromJsonError::ExpectedConstructorKeyValue)
				},
				_ => Err(FromJsonError::ExpectedConstructorKeyValue),
			}
		}

		fn expect_map_entry(value: &JValue) -> Result<MapDatumEntry, FromJsonError> {
			match value.as_object() {
				Some(map) => {
					let key = map.get("key").ok_or(FromJsonError::ExpectedDatumMapEntry)?;
					let key: Datum = TryFrom::try_from(key)?;
					let value = map.get("value").ok_or(FromJsonError::ExpectedDatumMapEntry)?;
					let value: Datum = TryFrom::try_from(value)?;
					Ok(MapDatumEntry { key, value })
				},
				_ => Err(FromJsonError::ExpectedDatumMapEntry),
			}
		}
		fn from_map(map: &JMap<String, JValue>) -> Result<Datum, FromJsonError> {
			let mut iter = map.into_iter();
			let first = iter.next();
			let second = iter.next();
			match first {
				Some((key, value)) => match key.as_str() {
					"int" => value
						.as_u64()
						.ok_or(FromJsonError::ExpectedIntegerDatumValue)
						.map(|int| int.to_datum()),
					"bytes" => {
						let str: &str =
							value.as_str().ok_or(FromJsonError::ExpectedHexEncodedBytes)?;
						let bytes: Vec<u8> =
							hex::decode(str).map_err(|_| FromJsonError::ExpectedHexEncodedBytes)?;
						Ok(bytes.to_datum())
					},
					"list" => {
						let list = expect_list(value)?;
						Ok(ListDatum(list))
					},
					"constructor" => {
						let constructor: u64 =
							value.as_u64().ok_or(FromJsonError::ExpectedConstructorKeyValue)?;
						let fields = expect_fields(second)?;
						Ok(ConstructorDatum { constructor, fields })
					},
					"fields" => {
						let fields = expect_list(value)
							.map_err(|_| FromJsonError::ExpectedFieldsKeyValue)?;
						let constructor = expect_constructor(second)?;
						Ok(ConstructorDatum { constructor, fields })
					},
					"map" => {
						let v: &Vec<JValue> =
							value.as_array().ok_or(FromJsonError::ExpectedArrayOfDatums)?;
						let entries: Vec<Result<MapDatumEntry, FromJsonError>> =
							v.iter().map(expect_map_entry).collect();
						let map: Result<Vec<MapDatumEntry>, FromJsonError> =
							entries.into_iter().collect();
						Ok(MapDatum(map?))
					},
					_ => Err(FromJsonError::ExpectedDatum),
				},
				None => Err(FromJsonError::ExpectedDatum),
			}
		}

		match value {
			JValue::Object(map) => from_map(map),
			_ => Err(FromJsonError::ExpectedDatum),
		}
	}
}

impl From<&Datum> for JValue {
	fn from(value: &Datum) -> Self {
		match value {
			IntegerDatum(int) => {
				json!({"int": int.to_i64().expect("integers exceeding i64 are not supported")})
			},
			ByteStringDatum(bytes) => json!({ "bytes": hex::encode(bytes) }),
			ListDatum(list) => {
				let list: Vec<JValue> = list.iter().map(Self::from).collect();
				json!({ "list": list })
			},
			ConstructorDatum { constructor, fields } => {
				let fields: Vec<JValue> = fields.iter().map(Self::from).collect();
				json!({"constructor": constructor, "fields": fields })
			},
			MapDatum(map) => {
				let map: Vec<JValue> = map
					.iter()
					.map(|e| json!({"key": Self::from(&e.key), "value": Self::from(&e.value)}))
					.collect();
				json!({ "map": map })
			},
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum FromJsonError {
	ExpectedArrayOfDatums,
	ExpectedHexEncodedBytes,
	ExpectedConstructorKeyValue,
	ExpectedDatum,
	ExpectedFieldsKeyValue,
	ExpectedIntegerDatumValue,
	ExpectedDatumMapEntry,
}

// tip: this function accepts Option<T> for any T: Datum
//      so it's better to call it with explicit type to avoid hard to
//      find bugs when types change
pub fn to_datum_cbor_bytes<T: ToDatum>(msg: T) -> Vec<u8> {
	minicbor::to_vec(msg.to_datum()).expect("Infallible error type never fails")
}
