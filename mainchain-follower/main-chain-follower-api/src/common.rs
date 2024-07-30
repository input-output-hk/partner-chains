#[cfg(feature = "serde")]
use derive_more::FromStr;
use sidechain_domain::*;
use std::fmt::{Debug, Display, Formatter};

extern crate alloc;

/// Timestamp - milliseconds since Unix Epoch
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(FromStr))]
pub struct Timestamp(pub u64);

impl Display for Timestamp {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl From<u64> for Timestamp {
	fn from(value: u64) -> Self {
		Timestamp(value)
	}
}

pub fn blake2b_28(data: &[u8]) -> [u8; MAINCHAIN_ADDRESS_HASH_LEN] {
	blake2b_simd::Params::new()
		.hash_length(MAINCHAIN_ADDRESS_HASH_LEN)
		.hash(data)
		.as_bytes()
		.try_into()
		.expect("hash output always has expected length")
}
