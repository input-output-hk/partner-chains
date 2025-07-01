//! Minimal Plutus data types and encoding implementation
//!
//! This crate implements the `Data` type used by [Untyped Plutus Core], a low-level language
//! used by Cardano smart contracts, along with traits for encoding Rust types as Plutus data
//! and Plutus data CBOR bytes.
//!
//! This crate is developed as part of the [Partner Chains toolkit] for use in [no_std] contexts
//! where existing alternatives ([uplc], [cardano-serialization-lib]) can not be used, and as
//! such is not intended to be feature-complete for general use.
//!
//! [Untyped Plutus Core]: https://plutonomicon.github.io/plutonomicon/uplc
//! [Partner Chains toolkit]: https://github.com/input-output-hk/partner-chains
//! [no_std]: https://doc.rust-lang.org/reference/names/preludes.html?highlight=no_std#r-names.preludes.extern.no_std
//! [uplc]: https://github.com/aiken-lang/aiken/tree/main/crates/uplc
//! [cardano-serialization-lib]: https://github.com/Emurgo/cardano-serialization-lib
#![no_std]
#![deny(missing_docs)]

mod cbor;

extern crate alloc;

use crate::Datum::*;
#[doc(hidden)]
pub use alloc::{vec, vec::Vec};
use core::fmt::{Debug, Formatter};
use num_bigint::BigInt;

/// Plutus datum type
#[derive(Clone, PartialEq)]
pub enum Datum {
	/// Integer value
	IntegerDatum(BigInt),
	/// Byte string value
	ByteStringDatum(Vec<u8>),
	/// Constructor value
	///
	/// A Plutus constructor datum consists of an ordered list of Plutus field values together
	/// with numeric constructor ID that marks the constructor variant used. These can only be
	/// interpreted against an external schema that specifies expected content of `fields` and
	/// their interpretation.
	ConstructorDatum {
		/// Constructor variant number
		constructor: u64,
		/// List of field values
		fields: Vec<Datum>,
	},
	/// List of values
	ListDatum(Vec<Datum>),
	/// Key-value mapping
	MapDatum(Vec<MapDatumEntry>),
}

/// Key-value pair stored in [Datum::MapDatum]
#[derive(Clone, Debug, PartialEq)]
pub struct MapDatumEntry {
	/// Key
	key: Datum,
	/// Value
	value: Datum,
}

/// Trait for types that can be encoded as a Plutus [Datum].
pub trait ToDatum {
	/// Encodes `self` to [Datum].
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

	/// Returns byte content if `self` is a [Datum::ByteStringDatum] and [None] otherwise.
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

/// Encodes a message as a Plutus [Datum] and returns [CBOR] encoding of this datum.
///
/// tip: this function accepts `Option<T>` for any `T: Datum`
///      so it's better to call it with explicit type to avoid hard to
///      find bugs when types change
///
/// [CBOR]: https://cbor.io
pub fn to_datum_cbor_bytes<T: ToDatum>(msg: T) -> Vec<u8> {
	minicbor::to_vec(msg.to_datum()).expect("Infallible error type never fails")
}
