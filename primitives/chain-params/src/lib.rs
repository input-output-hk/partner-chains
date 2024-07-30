#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec;
#[cfg(feature = "std")]
use clap::{arg, command, Parser};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use plutus::{Datum, ToDatum};
use plutus_datum_derive::*;
use scale_info::TypeInfo;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use sidechain_domain::{MainchainAddressHash, UtxoId};

/// Reference chain V1 parameters. Please do not depend on this structure, but rather implement your own.
/// Library code in this repository does not depend on this structure.
#[derive(
	Default, Debug, Clone, PartialEq, Eq, TypeInfo, Encode, Decode, ToDatum, MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(feature = "std", derive(Parser))]
#[cfg_attr(feature = "std", command(author, version, about, long_about = None))]
pub struct SidechainParams {
	#[cfg_attr(feature = "std", arg(long))]
	pub chain_id: u16,
	#[cfg_attr(feature = "std", arg(long))]
	pub genesis_committee_utxo: UtxoId,
	#[cfg_attr(feature = "std", arg(long))]
	pub threshold_numerator: u64,
	#[cfg_attr(feature = "std", arg(long))]
	pub threshold_denominator: u64,
	/// Obtained with cardano-cli address key-hash --payment-verification-key-file <vkey of chosen governance>
	#[cfg_attr(feature = "std", arg(long))]
	pub governance_authority: MainchainAddressHash,
}
