#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::fmt::Debug;
use alloc::string::String;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sidechain_domain::{byte_string::*, *};
use sp_inherents::*;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"govrnmap";

#[derive(Debug, Clone, PartialEq, Eq, TypeInfo, Encode, Decode, MaxEncodedLen, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MainChainScriptsV1 {
	pub validator_address: MainchainAddress,
	pub asset: AssetId,
}

/// Type describing a change made to a single key-value pair in the Governed Map.
#[derive(Decode, Encode, TypeInfo, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GovernedMapChangeV1 {
	pub key: String,
	pub new_value: Option<ByteString>,
}

#[derive(Decode, Encode, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(thiserror::Error))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Inherent missing for Governed Map pallet"))]
	InherentMissing,
	#[cfg_attr(feature = "std", error("Unexpected inherent for Governed Map pallet"))]
	InherentNotExpected,
	#[cfg_attr(
		feature = "std",
		error("Data in Governed Map pallet inherent differs from inherent data")
	)]
	IncorrectInherent,
	#[cfg_attr(feature = "std", error("Governed Map key {0} exceeds size bounds"))]
	KeyExceedsBounds(String),
	#[cfg_attr(feature = "std", error("Governed Map value {1:?} for key {0} exceeds size bounds"))]
	ValueExceedsBounds(String, ByteString),
	#[cfg_attr(feature = "std", error("Number of changes to the Governed Map exceeds the limit"))]
	TooManyChanges,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}
