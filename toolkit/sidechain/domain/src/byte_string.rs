//! Module providing various byte sequence wrapper types for use by the Partner Chains Toolkit crates

use alloc::string::ToString;
use alloc::vec::Vec;
use byte_string_derive::byte_string;
use core::fmt::{Debug, Display};
use core::ops::Deref;
use derive_where::derive_where;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::de::Error as DeError;
use serde::ser::Error as SerError;
use serde::{Deserialize, Serialize};
use sp_core::Get;
use sp_core::bounded::BoundedVec;

/// Wrapper for bytes that is serialized as hex string
/// To be used for binary data that we want to display nicely but don't have a specific type for
#[derive(Eq, Clone, PartialEq, TypeInfo, Default, Encode, Decode, PartialOrd, Ord)]
#[byte_string(debug)]
#[cfg_attr(feature = "std", byte_string(to_hex_string, decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
pub struct ByteString(pub Vec<u8>);

impl From<&[u8]> for ByteString {
	fn from(bytes: &[u8]) -> Self {
		Self(bytes.to_vec())
	}
}

impl From<Vec<u8>> for ByteString {
	fn from(vec: Vec<u8>) -> Self {
		Self(vec)
	}
}

impl Deref for ByteString {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Constant size variant of `ByteString` that's usable in a FRAME runtime
#[derive(
	Eq,
	Clone,
	PartialEq,
	TypeInfo,
	MaxEncodedLen,
	Encode,
	Decode,
	DecodeWithMemTracking,
	PartialOrd,
	Ord,
)]
#[byte_string(debug)]
#[byte_string(to_hex_string)]
#[cfg_attr(feature = "std", byte_string(decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
pub struct SizedByteString<const N: usize>(pub [u8; N]);

impl<const N: usize> From<[u8; N]> for SizedByteString<N> {
	fn from(value: [u8; N]) -> Self {
		Self(value)
	}
}

impl<const N: usize> TryFrom<Vec<u8>> for SizedByteString<N> {
	type Error = <[u8; N] as TryFrom<Vec<u8>>>::Error;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(SizedByteString(value.try_into()?))
	}
}

impl<const N: usize> Default for SizedByteString<N> {
	fn default() -> Self {
		Self([0; N])
	}
}

/// Bounded-length bytes representing a UTF-8 text string
#[derive(TypeInfo, Encode, DecodeWithMemTracking, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
#[derive_where(Clone, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct BoundedString<T: Get<u32>>(BoundedVec<u8, T>);

/// Macro that creates a [BoundedString], supporting string interpolation like [alloc::format].
#[macro_export]
macro_rules! bounded_str {
    ($($arg:expr)+) => {
		sidechain_domain::byte_string::BoundedString::try_from(format!($($arg)+).as_str()).unwrap()
    };
}

impl<T: Get<u32>> Decode for BoundedString<T> {
	fn decode<I: parity_scale_codec::Input>(
		input: &mut I,
	) -> Result<Self, parity_scale_codec::Error> {
		let bounded_vec: BoundedVec<u8, T> = Decode::decode(input)?;

		// Check if bytes are valid UTF-8 characters
		core::str::from_utf8(bounded_vec.as_slice())
			.map_err(|_err| parity_scale_codec::Error::from("UTF-8 decode failed"))?;

		Ok(Self(bounded_vec))
	}
}

impl<'a, T: Get<u32>> Deserialize<'a> for BoundedString<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'a>,
	{
		Ok(Self(
			BoundedVec::try_from(
				alloc::string::String::deserialize(deserializer)?.as_bytes().to_vec(),
			)
			.map_err(|_| D::Error::custom("Size limit exceeded"))?,
		))
	}
}

impl<T: Get<u32>> Serialize for BoundedString<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let str = alloc::string::String::from_utf8(self.0.to_vec())
			.map_err(|_| S::Error::custom("String is not valid UTF-8"))?;
		serializer.serialize_str(&str)
	}
}

impl<T: Get<u32>> Display for BoundedString<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(
			&alloc::string::String::from_utf8(self.0.to_vec()).map_err(|_| core::fmt::Error)?,
		)
	}
}

impl<T: Get<u32>> Debug for BoundedString<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.write_str(&alloc::format!("BoundedString<{}>({})", T::get(), self))
	}
}

impl<T: Get<u32>> TryFrom<&str> for BoundedString<T> {
	type Error = alloc::string::String;

	fn try_from(value: &str) -> Result<Self, Self::Error> {
		Ok(Self(BoundedVec::try_from(value.as_bytes().to_vec()).map_err(|_| value.to_string())?))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn decode_valid_utf8_string() {
		let original = "hello world".encode();

		let decoded = BoundedString::<sp_core::ConstU32<32>>::decode(&mut &*original).unwrap();
		assert_eq!(original, decoded.encode());
	}

	#[test]
	fn fail_to_decode_invalid_utf8_string() {
		let original = vec![0xC0, 0x80, 0xE0, 0xFF].encode(); // some invalid utf8 characters
		assert_eq!(
			BoundedString::<sp_core::ConstU32<32>>::decode(&mut &*original),
			Err(parity_scale_codec::Error::from("UTF-8 decode failed"))
		);

		use parity_scale_codec::DecodeWithMemLimit;

		// Safety check, due we leave derived DecodeWithMemTracking, so we want to ensure that it is also using Decode trait impl
		assert_eq!(
			BoundedString::<sp_core::ConstU32<32>>::decode_with_mem_limit(&mut &*original, 100),
			Err(parity_scale_codec::Error::from("UTF-8 decode failed"))
		);
	}
}
