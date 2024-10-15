//! Common types used in the Ogmios API.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct SlotLength {
	pub milliseconds: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct TimeSeconds {
	pub seconds: u64,
}
