use core::fmt::{Debug, Display};

use alloc::string::ToString;
use alloc::vec::Vec;
use byte_string_derive::byte_string;
use derive_where::derive_where;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::de::Error as DeError;
use serde::ser::Error as SerError;
use serde::{Deserialize, Serialize};
use sp_core::bounded::BoundedVec;
use sp_core::Get;

/// Wrapper for bytes that is serialized as hex string
/// To be used for binary data that we want to display nicely but
/// don't have a specific type for
#[derive(Eq, Clone, PartialEq, TypeInfo, Default, Encode, Decode, PartialOrd, Ord)]
#[byte_string(debug)]
#[cfg_attr(feature = "std", byte_string(to_hex_string, decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
pub struct ByteString(pub Vec<u8>);

impl From<Vec<u8>> for ByteString {
	fn from(vec: Vec<u8>) -> Self {
		Self(vec)
	}
}

// Constant size variant of `ByteString` that's usable as a runtime type
#[derive(Eq, Clone, PartialEq, TypeInfo, MaxEncodedLen, Encode, Decode)]
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

/// Byte-encoded text string with bounded length
#[derive(TypeInfo, Encode, Decode, MaxEncodedLen)]
#[scale_info(skip_type_params(T))]
#[derive_where(Clone, PartialEq, Eq, Default)]
pub struct BoundedString<T: Get<u32>>(pub BoundedVec<u8, T>);

impl<T: Get<u32>> TryFrom<Vec<u8>> for BoundedString<T> {
	type Error = <BoundedVec<u8, T> as TryFrom<Vec<u8>>>::Error;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		Ok(Self(value.try_into()?))
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
