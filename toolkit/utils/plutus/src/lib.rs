#![no_std]

mod cbor;

extern crate alloc;

use crate::Datum::*;
pub use alloc::{vec, vec::Vec};
use core::fmt::{Debug, Formatter};
use num_bigint::BigInt;

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

// tip: this function accepts Option<T> for any T: Datum
//      so it's better to call it with explicit type to avoid hard to
//      find bugs when types change
pub fn to_datum_cbor_bytes<T: ToDatum>(msg: T) -> Vec<u8> {
	minicbor::to_vec(msg.to_datum()).expect("Infallible error type never fails")
}
