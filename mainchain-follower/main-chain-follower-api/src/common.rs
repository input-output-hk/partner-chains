#[cfg(feature = "serde")]
use derive_more::FromStr;
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
