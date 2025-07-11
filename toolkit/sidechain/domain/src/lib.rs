//! # Partner Chains domain types
//!
//! This crate defines common domain and utility types used in the Partner Chain Toolkit.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

pub mod byte_string;
pub mod crypto;
pub mod mainchain_epoch;

extern crate alloc;
extern crate core;
extern crate num_derive;

pub use alloc::collections::btree_map::BTreeMap;
#[cfg(feature = "std")]
use alloc::format;
pub use alloc::vec::Vec;
use alloc::{str::FromStr, string::String, string::ToString, vec};
use byte_string_derive::byte_string;
use core::{
	fmt::{Display, Formatter},
	ops::Deref,
};
use crypto::blake2b;
use derive_more::{From, Into};
use num_derive::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen, WrapperTypeEncode};
use plutus_datum_derive::*;
use scale_info::TypeInfo;
use sp_core::{
	ConstU32,
	bounded::BoundedVec,
	crypto::{
		KeyTypeId,
		key_types::{AURA, GRANDPA},
	},
	ecdsa, ed25519, sr25519,
};
#[cfg(feature = "serde")]
use {
	derive_more::FromStr,
	serde::{Deserialize, Deserializer, Serialize, Serializer},
};

/// The number of main chain epochs back a Partner Chain queries for committee selection inputs.
/// This offset is necessary to ensure that data is present and stable.
const DATA_MC_EPOCH_OFFSET: u32 = 2;

/// Shifts given epoch back by [DATA_MC_EPOCH_OFFSET] accounting for underflow.
pub fn offset_data_epoch(epoch: &McEpochNumber) -> Result<McEpochNumber, u32> {
	Ok(McEpochNumber(epoch.0.checked_sub(DATA_MC_EPOCH_OFFSET).ok_or(DATA_MC_EPOCH_OFFSET)?))
}

#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	Hash,
	TypeInfo,
	Ord,
	PartialOrd,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
/// Cardano epoch number. In range [0, 2^31-1].
pub struct McEpochNumber(pub u32);

impl McEpochNumber {
	/// Returns next Cardano epoch number
	pub fn next(&self) -> Self {
		Self(&self.0 + 1)
	}
}

impl Display for McEpochNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u32::fmt(&self.0, f)
	}
}
#[derive(
	Default,
	Clone,
	Copy,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// Amount of Lovelace (which is a fraction of 1 ADA) staked/locked on Cardano
pub struct StakeDelegation(pub u64);

impl StakeDelegation {
	/// Checks if stake delegation is zero
	pub fn is_zero(&self) -> bool {
		self.0 == 0
	}
}

#[derive(
	Default,
	Clone,
	Copy,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	PartialEq,
	Eq,
	From,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// The amount of a Cardano native token
pub struct NativeTokenAmount(pub u128);

impl Display for NativeTokenAmount {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u128::fmt(&self.0, f)
	}
}

#[derive(
	Default,
	Clone,
	Copy,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	scale_info::TypeInfo,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
/// Cardano block number. In range [0, 2^31-1].
pub struct McBlockNumber(pub u32);

impl Display for McBlockNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u32::fmt(&self.0, f)
	}
}

#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
/// Cardano slot number. In range [0, 2^63-1].
pub struct McSlotNumber(pub u64);

impl Display for McSlotNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u64::fmt(&self.0, f)
	}
}

#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Hash,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
/// Partner Chain slot number
pub struct ScSlotNumber(pub u64);

/// Data describing a Cardano block
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MainchainBlock {
	/// Block number
	pub number: McBlockNumber,
	/// Block hash
	pub hash: McBlockHash,
	/// Block's epoch number
	pub epoch: McEpochNumber,
	/// Block's slot number
	pub slot: McSlotNumber,
	/// Block timestamp
	pub timestamp: u64, // seconds since UNIX_EPOCH
}

#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	PartialOrd,
	Ord,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// An index of transaction in a block. In range [0, 2^31-1].
pub struct McTxIndexInBlock(pub u32);

#[cfg(feature = "serde")]
impl FromStr for McTxIndexInBlock {
	type Err = sp_std::num::ParseIntError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let parsed = u32::from_str(s)?;
		let _check_overflow = i32::from_str(s)?;
		Ok(Self(parsed))
	}
}

/// Maximum length of a Cardano address in UTF-8 bytes
const MAX_MAINCHAIN_ADDRESS_BYTES: u32 = 120;

/// Wraps UTF-8 bytes of Mainchain Address in bech32 format.
/// Example: utf-8 bytes of "addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc" are
/// "0x616464725f7465737431777a35716337666b327061743030353877347a77766b77333579747074656a336e7563336a65326b6774616e356471337274347363"
#[derive(
	Clone, Default, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
#[byte_string(debug)]
pub struct MainchainAddress(BoundedVec<u8, ConstU32<MAX_MAINCHAIN_ADDRESS_BYTES>>);

impl MainchainAddress {
	/// Returns raw bytes of this Cardano address
	pub fn bytes(&self) -> Vec<u8> {
		self.0.to_vec()
	}
}

#[cfg(feature = "serde")]
impl FromStr for MainchainAddress {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let bytes: Vec<u8> = s.as_bytes().to_vec();
		let bounded = BoundedVec::try_from(bytes).map_err(|_| "Invalid length")?;
		Ok(MainchainAddress(bounded))
	}
}

impl Display for MainchainAddress {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let s = String::from_utf8(self.0.to_vec())
			.expect("MainchainAddressString is always properly encoded UTF-8");
		write!(f, "{}", s)
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for MainchainAddress {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let s = String::from_utf8(self.0.to_vec()).expect("MainchainAddress is always valid UTF-8");
		serializer.serialize_str(&s)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MainchainAddress {
	/// Deserialized MainchainAddress from both hexstring of ASCII bytes and plain String
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		let bytes = sp_core::bytes::from_hex(&s).unwrap_or_else(|_| s.as_bytes().to_vec());
		let bounded = BoundedVec::try_from(bytes)
			.map_err(|_| serde::de::Error::custom("MainchainAddress is too long"))?;
		Ok(MainchainAddress(bounded))
	}
}

/// Cardano Policy Id is a 224 bits blake2b hash.
const POLICY_ID_LEN: usize = 28;
#[derive(
	Clone,
	Default,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	ToDatum,
	TypeInfo,
	MaxEncodedLen,
	Hash,
)]
#[byte_string(debug, decode_hex, hex_serialize, hex_deserialize)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
/// Cardano Policy Id
pub struct PolicyId(pub [u8; POLICY_ID_LEN]);

#[cfg(feature = "std")]
impl Display for PolicyId {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.to_hex_string())
	}
}

/// Cardano script hash
pub type ScriptHash = PolicyId;

/// Maximum length of a Cardano native asset's name in UTF-8 bytes
pub const MAX_ASSET_NAME_LEN: u32 = 32;

/// Cardano native asset name
#[derive(
	Clone, Default, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen,
)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
pub struct AssetName(pub BoundedVec<u8, ConstU32<MAX_ASSET_NAME_LEN>>);

impl AssetName {
	/// Constructs an empty [AssetName]
	pub fn empty() -> Self {
		Self(BoundedVec::new())
	}
}

#[cfg(feature = "std")]
impl Display for AssetName {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}", self.to_hex_string())
	}
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Default,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// Full data identifying a Cardano native asset
pub struct AssetId {
	/// Policy ID
	pub policy_id: PolicyId,
	/// Asset name
	pub asset_name: AssetName,
}

#[cfg(feature = "std")]
impl FromStr for AssetId {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s.split_once(".") {
			Some((policy_id, asset_name)) => {
				let policy_id = PolicyId::from_str(policy_id)
					.map_err(|e| format!("{} is invalid Policy ID: {}", policy_id, e))?;
				let asset_name = AssetName::from_str(asset_name)
					.map_err(|e| format!("{} is invalid Asset Name: {}", asset_name, e))?;
				Ok(Self { policy_id, asset_name })
			},
			None => {
				Err("AssetId should be <hex encoded Policy ID>.<hex encoded Asset Name>"
					.to_string())
			},
		}
	}
}

/// Length of Cardano stake pool key
const STAKE_POOL_PUBLIC_KEY_LEN: usize = 32;

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Hash,
	Ord,
	PartialOrd,
)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
/// Cardano stake pool public key (ed25519)
pub struct StakePoolPublicKey(pub [u8; STAKE_POOL_PUBLIC_KEY_LEN]);

impl StakePoolPublicKey {
	/// Computes the blake2b_224 hash of this Cardano stake pool public key
	pub fn hash(&self) -> MainchainKeyHash {
		MainchainKeyHash::from_vkey(&self.0)
	}
}

impl TryFrom<Vec<u8>> for StakePoolPublicKey {
	type Error = &'static str;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		<[u8; 32]>::try_from(value)
			.map_err(|_| "Mainchain public key must be 32 bytes long")
			.map(StakePoolPublicKey)
	}
}

/// Length of Cardano staking public key
const STAKE_PUBLIC_KEY_LEN: usize = 32;

/// Cardano staking public key (ed25519)
#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, MaxEncodedLen, Hash,
)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct StakePublicKey(pub [u8; STAKE_PUBLIC_KEY_LEN]);

impl StakePublicKey {
	/// Computes the blake2b_224 hash of this Cardano staking public key
	pub fn hash(&self) -> MainchainKeyHash {
		MainchainKeyHash(blake2b(&self.0))
	}
}

/// Length of Cardano key hash
const MAINCHAIN_KEY_HASH_LEN: usize = 28;

#[derive(
	Clone,
	Copy,
	Decode,
	DecodeWithMemTracking,
	Default,
	Encode,
	Hash,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	TypeInfo,
)]
#[byte_string(debug)]
#[cfg_attr(feature = "std", byte_string(to_hex_string, decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
/// blake2b_224 hash of a Cardano Verification (Public) Key.
/// It can be a hash of Payment Verification, Payment Extended Verification, Stake Pool Verification Key or Staking Verification Key.
pub struct MainchainKeyHash(pub [u8; MAINCHAIN_KEY_HASH_LEN]);

impl Display for MainchainKeyHash {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let hash = sp_core::hexdisplay::HexDisplay::from(&self.0);
		write!(f, "0x{}", hash)
	}
}

impl MainchainKeyHash {
	/// Computes the blake2b_224 hash of the given ed25519 public key bytes
	pub fn from_vkey(vkey: &[u8; 32]) -> Self {
		Self(blake2b(vkey))
	}
}

/// Length of Cardano signature (EDDSA)
pub const MAINCHAIN_SIGNATURE_LEN: usize = 64;

#[derive(Clone, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
#[byte_string(debug, hex_serialize, decode_hex)]
/// Cardano signature type (EDDSA)
///
/// WARNING: This type needs to be backwards compatible with a legacy schema wrapping `Vec<u8>`.
///          Because of this, it is not handled correctly by PolkadotJS. If you need to accept
///          this type as extrinsic argument, use raw `[u8; MAINCHAIN_SIGNATURE_LEN]` instead.
pub struct MainchainSignature(pub [u8; MAINCHAIN_SIGNATURE_LEN]);

impl From<[u8; MAINCHAIN_SIGNATURE_LEN]> for MainchainSignature {
	fn from(raw: [u8; MAINCHAIN_SIGNATURE_LEN]) -> Self {
		Self(raw)
	}
}

impl WrapperTypeEncode for MainchainSignature {}
impl Deref for MainchainSignature {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl Decode for MainchainSignature {
	fn decode<I: parity_scale_codec::Input>(
		input: &mut I,
	) -> Result<Self, parity_scale_codec::Error> {
		let vec: Vec<u8> = Decode::decode(input)?;
		let arr = vec.try_into().map_err(|_| "Incorrect MainchainSignature size")?;
		Ok(MainchainSignature(arr))
	}
}

impl MainchainSignature {
	/// Verifies whether `self` is a valid signature of `signed_message` for `public_key`
	pub fn verify(&self, public_key: &StakePoolPublicKey, signed_message: &[u8]) -> bool {
		let mainchain_signature = ed25519::Signature::from(self.0);

		sp_io::crypto::ed25519_verify(
			&mainchain_signature,
			signed_message,
			&ed25519::Public::from(public_key.0),
		)
	}
}

/// Length of Cardano staking key signature (EDDSA)
pub const STAKE_KEY_SIGNATURE_LEN: usize = 64;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, Hash, TypeInfo)]
#[byte_string(debug, hex_serialize, decode_hex)]
/// Cardano staking key signature type (EDDSA)
pub struct StakeKeySignature(pub [u8; STAKE_KEY_SIGNATURE_LEN]);

impl From<[u8; STAKE_KEY_SIGNATURE_LEN]> for StakeKeySignature {
	fn from(raw: [u8; STAKE_KEY_SIGNATURE_LEN]) -> Self {
		Self(raw)
	}
}

impl StakeKeySignature {
	/// Verifies whether `self` is a valid signature of `message` for `public_key`
	pub fn verify(&self, public_key: &StakePublicKey, message: &[u8]) -> bool {
		let signature = ed25519::Signature::from(self.0);
		sp_io::crypto::ed25519_verify(&signature, message, &ed25519::Public::from(public_key.0))
	}
}

#[derive(
	Clone,
	Copy,
	Debug,
	Encode,
	Decode,
	DecodeWithMemTracking,
	PartialEq,
	TypeInfo,
	ToDatum,
	MaxEncodedLen,
	Default,
	PartialOrd,
	Ord,
	Eq,
	Zero,
	One,
	NumOps,
	Num,
	From,
	Into,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// Partner Chain epoch number
pub struct ScEpochNumber(pub u64);

impl Display for ScEpochNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u64::fmt(&self.0, f)
	}
}

impl ScEpochNumber {
	/// Returns next epoch number
	pub fn next(&self) -> Self {
		Self(self.0 + 1)
	}
	/// Returns previous epoch number accounting for underflow
	pub fn prev(&self) -> Option<Self> {
		self.0.checked_sub(1).map(Self)
	}
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	ToDatum,
	TypeInfo,
	PartialOrd,
	Ord,
	Hash,
)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex, as_ref)]
/// Partner Chain public key
///
/// This public key is used as the identity of a Partner Chain network participant on a specific Partner Chain,
/// ie. a network participant can use different [SidechainPublicKey] for each Partner Chain they are active on
/// as opposed to [CrossChainPublicKey].
pub struct SidechainPublicKey(pub Vec<u8>);

impl From<ecdsa::Public> for SidechainPublicKey {
	fn from(value: ecdsa::Public) -> Self {
		Self(value.0.to_vec())
	}
}

/// CBOR bytes of Cardano Transaction.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct TransactionCbor(pub Vec<u8>);

/// CBOR bytes of Cardano VKeyWitness.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct VKeyWitnessCbor(pub Vec<u8>);

/// Cross-chain signature type (ECDSA) created using [SidechainPublicKey]
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct SidechainSignature(pub Vec<u8>);

/// Cross-chain public key (ECDSA)
///
/// This public key is used as the universal identity of Partner Chain network participants across all Partner Chains.
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize, hex_deserialize)]
pub struct CrossChainPublicKey(pub Vec<u8>);

impl CrossChainPublicKey {
	/// Computes the blake2b_224 hash of this cross-chain public key
	pub fn hash(&self) -> CrossChainKeyHash {
		CrossChainKeyHash(blake2b(&self.0))
	}
}

impl From<k256::PublicKey> for CrossChainPublicKey {
	fn from(value: k256::PublicKey) -> Self {
		Self(value.to_sec1_bytes().to_vec())
	}
}

impl From<CrossChainPublicKey> for k256::PublicKey {
	fn from(value: CrossChainPublicKey) -> Self {
		k256::PublicKey::from_sec1_bytes(&value.0)
			.expect("CrossChainPublicKey converts to valid secp256k1::PublicKey")
	}
}

/// Length of the cross-chain public key hash
const CROSS_CHAIN_KEY_HASH_LEN: usize = 28;

#[derive(
	Clone,
	Copy,
	Decode,
	DecodeWithMemTracking,
	Default,
	Encode,
	Hash,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	TypeInfo,
)]
#[byte_string(debug, to_hex_string)]
#[cfg_attr(feature = "std", byte_string(decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
/// blake2b_224 hash of a cross-chain public key
pub struct CrossChainKeyHash(pub [u8; CROSS_CHAIN_KEY_HASH_LEN]);

impl Display for CrossChainKeyHash {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		f.write_str(&self.to_hex_string())
	}
}

/// Cross-chain signature created using [CrossChainPublicKey]
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct CrossChainSignature(pub Vec<u8>);

impl CrossChainSignature {
	/// Verifies that `self` is a valid signature of `data` for `cross_chain_pubkey`
	pub fn verify(
		&self,
		cross_chain_pubkey: &CrossChainPublicKey,
		data: &[u8],
	) -> Result<(), k256::ecdsa::signature::Error> {
		use k256::ecdsa::signature::Verifier;

		let vkey = k256::ecdsa::VerifyingKey::from_sec1_bytes(&cross_chain_pubkey.0[..])?;
		let signature = k256::ecdsa::Signature::from_slice(&self.0[..])?;
		vkey.verify(data, &signature)
	}
}

/// Length of Cardano epoch nonce
const EPOCH_NONCE_LEN: usize = 32;

/// Cardano epoch nonce
///
/// This value is a 32-byte hash generated at the start of each epoch on Cardano using
/// a verifiable random function as part of normal chain operation by Cardano block producers.
/// Because it is subject to Cardano's consensus mechanism and has strong cryptographic guarantees,
/// this value can be used as a tamper-proof shared randomness seed by Partner Chain Toolkit components.
#[derive(Default, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct EpochNonce(pub Vec<u8>);

impl EpochNonce {
	/// Returns epoch nonce as byte array.
	pub fn as_array(&self) -> [u8; EPOCH_NONCE_LEN] {
		let mut epoch_nonce = self.0.clone();
		epoch_nonce.resize_with(32, || 0);
		epoch_nonce.try_into().expect("Should never fail after being resized")
	}
}

#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	ToDatum,
	TypeInfo,
	MaxEncodedLen,
	Hash,
)]
/// Identifies a Cardano UTxO (unspent transaction output)
///
/// A UTxO is uniquely identified by the hash of the transaction that produced it and its (zero-based)
/// index in the transaction's output.
///
/// Standard semi-human-readable encoding of a UTxO id uses a hash sign to divide the two components:
/// `0000000000000000000000000000000000000000000000000000000000000000#0`
pub struct UtxoId {
	/// Transaction hash
	pub tx_hash: McTxHash,
	/// Output index
	pub index: UtxoIndex,
}

impl UtxoId {
	/// Creates new [UtxoId] from primitive type arguments
	pub const fn new(hash: [u8; TX_HASH_SIZE], index: u16) -> UtxoId {
		UtxoId { tx_hash: McTxHash(hash), index: UtxoIndex(index) }
	}
}

#[cfg(feature = "serde")]
impl Serialize for UtxoId {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_string())
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for UtxoId {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		alloc::string::String::deserialize(deserializer).and_then(|string| {
			Self::from_str(&string).map_err(|err| serde::de::Error::custom(err.to_string()))
		})
	}
}

#[cfg(feature = "serde")]
impl FromStr for UtxoId {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let split: Vec<&str> = s.split('#').collect();
		let &[hash_str, index_str] = split.as_slice() else {
			return Err("UtxoId string must conform to format: '<hash>#<index>'");
		};

		Ok(UtxoId {
			tx_hash: McTxHash::from_str(hash_str)
				.map_err(|_| "invalid string input for McTxHash")?,
			index: UtxoIndex::from_str(index_str)
				.map_err(|_| "invalid string input for OutputIndex")?,
		})
	}
}

impl Display for UtxoId {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let hash = sp_core::hexdisplay::HexDisplay::from(&self.tx_hash.0);
		write!(f, "{}#{}", hash, self.index.0)
	}
}

#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	PartialOrd,
	Ord,
	ToDatum,
	TypeInfo,
	MaxEncodedLen,
	Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// An index of output of a transaction. In range [0, 2^15-1].
pub struct UtxoIndex(pub u16);

#[cfg(feature = "serde")]
impl FromStr for UtxoIndex {
	type Err = sp_std::num::ParseIntError;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let parsed = u16::from_str(s)?;
		let _check_overflow = i16::from_str(s)?;
		Ok(Self(parsed))
	}
}

/// Size of a Cardano transaction hash
pub const TX_HASH_SIZE: usize = 32;

#[derive(
	Default,
	Copy,
	Clone,
	Hash,
	PartialEq,
	Eq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	ToDatum,
	TypeInfo,
	MaxEncodedLen,
)]
#[byte_string(debug, from_bytes, decode_hex, hex_serialize, hex_deserialize)]
#[constructor_datum]
/// Cardano transaction hash
///
/// This hash uniquely identifies a transaction in the Cardano ledger.
pub struct McTxHash(pub [u8; TX_HASH_SIZE]);

impl TryFrom<Vec<u8>> for McTxHash {
	type Error = &'static str;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		<[u8; 32]>::try_from(value)
			.map_err(|_| "McTxHash must be 32 bytes long")
			.map(McTxHash)
	}
}

#[derive(
	Default,
	Clone,
	Decode,
	DecodeWithMemTracking,
	Encode,
	PartialEq,
	Eq,
	TypeInfo,
	MaxEncodedLen,
	Hash,
)]
#[byte_string(debug, decode_hex, hex_serialize, hex_deserialize)]
/// Cardano block hash
///
/// This hash uniquely identifies a Cardano block
pub struct McBlockHash(pub [u8; 32]);

impl Display for McBlockHash {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let hash = sp_core::hexdisplay::HexDisplay::from(&self.0);
		write!(f, "{}", hash)
	}
}

/// Extended information about a UTxO in Cardano ledger
#[derive(
	Default, Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UtxoInfo {
	/// Output ID
	pub utxo_id: UtxoId,
	/// Epoch number in which the output was produced
	pub epoch_number: McEpochNumber,
	/// Block number in which the output was produced
	pub block_number: McBlockNumber,
	/// Slot number in which the output was produced
	pub slot_number: McSlotNumber,
	/// Index in block of the transaction that produced the output
	pub tx_index_within_block: McTxIndexInBlock,
}

/// Key type used for ordering transaction outputs
///
/// This ordering key is used in contexts where a common ordering of the data must be used
/// by all nodes participating in a Partner Chain due to it being subject to consensus.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtxoInfoOrderingKey {
	/// Block number on which the output was created
	pub block_number: McBlockNumber,
	/// Index within the block of the transaction that created the output
	pub tx_index_within_block: McTxIndexInBlock,
	/// Index of the output in the transaction outputs
	pub utxo_id_index: UtxoIndex,
}

impl UtxoInfo {
	/// Returns the ordering key for this UTxO
	pub fn ordering_key(&self) -> UtxoInfoOrderingKey {
		UtxoInfoOrderingKey {
			block_number: self.block_number,
			tx_index_within_block: self.tx_index_within_block,
			utxo_id_index: self.utxo_id.index,
		}
	}
}

/// Type of Cardano network
///
/// Cardano defines two network types:
/// - mainnet: the unique, production Cardano network
/// - testnet: various public and private testnets. These testnets are further differentiated
///            by their respective "testnet magic" numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum NetworkType {
	/// The Cardano mainnet (unique network)
	Mainnet,
	/// A Cardano testnet
	#[default]
	Testnet,
}

#[cfg(feature = "std")]
impl std::fmt::Display for NetworkType {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let str = match self {
			Self::Mainnet => "mainnet",
			Self::Testnet => "testnet",
		};
		write!(f, "{}", str)
	}
}

/// Cardano SPO registration data
///
/// This data describes a single registration done by a Cardano SPO for the sake of being considered
/// for selection to the block producing committee on a given Partner Chain.
///
/// This registration is represented as a UTxO in the Cardano ledger containing a Plutus datum with
/// public keys that are being registered, together with signatures that prove the registrant's
/// control of these keys.
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct RegistrationData {
	/// UTXO that is an input parameter to the registration transaction
	pub registration_utxo: UtxoId,
	/// Signature confirming the registrant's ownership of `sidechain_pub_key`
	pub sidechain_signature: SidechainSignature,
	/// Signature confirming the registrant's ownership of the main chain public key used in the registration
	pub mainchain_signature: MainchainSignature,
	/// Signature confirming the registrant's ownership of `cross_chain_pub_key`
	pub cross_chain_signature: CrossChainSignature,
	/// Registering SPO's sidechain public key
	pub sidechain_pub_key: SidechainPublicKey,
	/// Registering SPO's cross-chain public key
	pub cross_chain_pub_key: CrossChainPublicKey,
	/// Information about the UTxO containing the registration data
	pub utxo_info: UtxoInfo,
	/// List of inputs to the registration transaction
	pub tx_inputs: Vec<UtxoId>,
	/// Registering SPO's additional keys
	pub keys: CandidateKeys,
}

/// Information about an Authority Candidate's Registrations at some block.
#[derive(Debug, Clone, Encode, Decode, DecodeWithMemTracking, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CandidateRegistrations {
	/// Stake pool public key of the registering Cardano SPO
	pub stake_pool_public_key: StakePoolPublicKey,
	/// List of registrations done by the registering Cardano SPO
	pub registrations: Vec<RegistrationData>,
	/// Stake delegation of the registering Cardano SPO
	pub stake_delegation: Option<StakeDelegation>,
}

impl CandidateRegistrations {
	/// Creates a new [CandidateRegistrations] from its members
	pub fn new(
		stake_pool_public_key: StakePoolPublicKey,
		stake_delegation: Option<StakeDelegation>,
		registrations: Vec<RegistrationData>,
	) -> Self {
		Self { stake_pool_public_key, registrations, stake_delegation }
	}

	/// Return the stake pool public key of the registering SPO
	pub fn mainchain_pub_key(&self) -> &StakePoolPublicKey {
		&self.stake_pool_public_key
	}

	/// Return the list of registrations of the SPO
	pub fn registrations(&self) -> &[RegistrationData] {
		&self.registrations
	}
}

/// Sr25519 public key used by Aura consensus algorithm. Not validated
#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialOrd, Ord, Hash,
)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct AuraPublicKey(pub Vec<u8>);
impl AuraPublicKey {
	/// Attempts to cast this public key to a valid [sr25519::Public]
	pub fn try_into_sr25519(&self) -> Option<sr25519::Public> {
		Some(sr25519::Public::from_raw(self.0.clone().try_into().ok()?))
	}
}

impl From<sr25519::Public> for AuraPublicKey {
	fn from(value: sr25519::Public) -> Self {
		Self(value.0.to_vec())
	}
}

impl From<AuraPublicKey> for CandidateKey {
	fn from(value: AuraPublicKey) -> Self {
		Self { id: AURA.0, bytes: value.0 }
	}
}

/// Ed25519 public key used by the Grandpa finality gadget. Not validated
#[derive(
	Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialOrd, Ord, Hash,
)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct GrandpaPublicKey(pub Vec<u8>);
impl GrandpaPublicKey {
	/// Attempts to cast this public key to a valid [ed25519::Public]
	pub fn try_into_ed25519(&self) -> Option<ed25519::Public> {
		Some(ed25519::Public::from_raw(self.0.clone().try_into().ok()?))
	}
}

impl From<ed25519::Public> for GrandpaPublicKey {
	fn from(value: ed25519::Public) -> Self {
		Self(value.0.to_vec())
	}
}

impl From<GrandpaPublicKey> for CandidateKey {
	fn from(value: GrandpaPublicKey) -> Self {
		Self { id: GRANDPA.0, bytes: value.0 }
	}
}

#[derive(
	Debug,
	Clone,
	PartialEq,
	Decode,
	DecodeWithMemTracking,
	Encode,
	MaxEncodedLen,
	TypeInfo,
	Eq,
	Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// Parameter controlling the number and proportion of registered and permissioned candidates
/// selected into a Partner Chain committee, used by the Ariadne family of selection algorithms.
///
/// The core idea behind the D-Param is to enable a Partner Chain to bootstrap its operation by
/// relying on a hand-picked set of trusted block producers for security, and to later incrementally
/// shift block production onto trustless network participants as the chain grows and it becomes
/// harder for malicious actors to manipulate the chain.
pub struct DParameter {
	/// Expected number of permissioned candidates selected for a committee
	pub num_permissioned_candidates: u16,
	/// Expected number of registered candidates selected for a committee
	pub num_registered_candidates: u16,
}

impl DParameter {
	/// Creates a new [DParameter] from member values
	pub fn new(num_permissioned_candidates: u16, num_registered_candidates: u16) -> Self {
		Self { num_permissioned_candidates, num_registered_candidates }
	}
}

/// Opaque key bytes with a 4 bytes identifier
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Decode,
	DecodeWithMemTracking,
	Encode,
	TypeInfo,
	PartialOrd,
	Ord,
	Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CandidateKey {
	/// Key type id
	pub id: [u8; 4],
	/// Bytes of the key
	pub bytes: Vec<u8>,
}

/// Key type id of Partner Chains cross-chain key, used with ECDSA cryptography
pub const CROSS_CHAIN_KEY_TYPE_ID: KeyTypeId = KeyTypeId(*b"crch");

impl CandidateKey {
	/// Constructor
	pub fn new(id: KeyTypeId, bytes: Vec<u8>) -> Self {
		Self { id: id.0, bytes }
	}
}

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Decode,
	DecodeWithMemTracking,
	Encode,
	TypeInfo,
	PartialOrd,
	Ord,
	Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// Bytes of CandidateKeys that come from Cardano or other input.
pub struct CandidateKeys(pub Vec<CandidateKey>);

impl CandidateKeys {
	/// Gets copy of key bytes identified by given id
	pub fn find(&self, id: KeyTypeId) -> Option<Vec<u8>> {
		self.0
			.iter()
			.find_map(|e| if e.id == id.0 { Some(e.bytes.clone()) } else { None })
	}

	/// Gets copy of key bytes identified by given id or empty bytes if key is not present
	pub fn find_or_empty(&self, id: KeyTypeId) -> Vec<u8> {
		self.find(id).unwrap_or_default()
	}
}

impl From<Vec<([u8; 4], Vec<u8>)>> for CandidateKeys {
	fn from(value: Vec<([u8; 4], Vec<u8>)>) -> Self {
		Self(value.into_iter().map(|(id, bytes)| CandidateKey { id, bytes }).collect())
	}
}

#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	Decode,
	DecodeWithMemTracking,
	Encode,
	TypeInfo,
	PartialOrd,
	Ord,
	Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
/// Information about a permissioned committee member candidate
///
/// Permissioned candidates are nominated by the Partner Chain's governance authority to be
/// eligible for participation in block producer committee without controlling any ADA stake
/// on Cardano and registering as SPOs.
pub struct PermissionedCandidateData {
	/// Sidechain public key of the permissioned candidate
	pub sidechain_public_key: SidechainPublicKey,
	/// Additional keys of the permissioned candidate
	pub keys: CandidateKeys,
}

/// Cardano SPO registration. This is a stripped-down version of [RegistrationData].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CandidateRegistration {
	/// Information on ADA stake pool ownership
	pub stake_ownership: AdaBasedStaking,
	/// Registering SPO's sidechain public key
	pub partner_chain_pub_key: SidechainPublicKey,
	/// Signature confirming registering SPO's ownership of `partner_chain_pub_key`
	pub partner_chain_signature: SidechainSignature,
	/// Hash of the registering SPO's Cardano public key
	pub own_pkh: MainchainKeyHash,
	/// UTxO containing the registration data
	pub registration_utxo: UtxoId,
	/// Additional keys of the registered candidate
	pub keys: CandidateKeys,
}

impl CandidateRegistration {
	/// Checks whether `self` and `other` contain the same keys
	pub fn matches_keys(&self, other: &Self) -> bool {
		self.stake_ownership == other.stake_ownership
			&& self.partner_chain_pub_key == other.partner_chain_pub_key
			&& self.partner_chain_signature == other.partner_chain_signature
			// TODO: sort before comparisone
			&& self.keys == other.keys
	}
}

/// Information on ADA stake pool ownership
///
/// AdaBasedStaking is a variant of Plutus type StakeOwnership. The other variant, TokenBasedStaking, is not supported.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AdaBasedStaking {
	/// Public key of the stake pool operator
	pub pub_key: StakePoolPublicKey,
	/// Signature confirming ownership of `pub_key`
	pub signature: MainchainSignature,
}

#[cfg(test)]
mod tests {
	use super::*;
	use core::str::FromStr;
	use hex_literal::hex;

	#[test]
	fn main_chain_address_string_serialize_deserialize_round_trip() {
		let address = MainchainAddress::from_str(
			"addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc",
		)
		.unwrap();
		let serialized = serde_json::to_value(&address).unwrap();
		assert_eq!(
			serialized,
			serde_json::json!("addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc")
		);
		let deserialized = serde_json::from_value(serialized).unwrap();
		assert_eq!(address, deserialized);
	}

	#[test]
	fn main_chain_address_deserialization_of_hex_encoded_bytes() {
		let address = MainchainAddress::from_str("addr_test1wz5q").unwrap();
		let serialized = serde_json::json!("0x616464725f7465737431777a3571");
		assert_eq!(address, serde_json::from_value(serialized).unwrap());
		let serialized = serde_json::json!("616464725f7465737431777a3571");
		assert_eq!(address, serde_json::from_value(serialized).unwrap());
	}

	#[test]
	fn main_chain_address_string_from_str_to_string_round_trip() {
		let address = MainchainAddress::from_str(
			"addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc",
		)
		.unwrap();
		let str = address.to_string();
		let from_str = MainchainAddress::from_str(&str).unwrap();
		assert_eq!(address, from_str);
	}

	#[test]
	fn main_chain_signature_should_be_backward_compatible_with_vec() {
		#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq, Hash)]
		#[byte_string(debug, hex_serialize, decode_hex)]
		struct LegacyMCSignature(pub Vec<u8>);

		let legacy_encoded = LegacyMCSignature(vec![10; 64]).encode();

		let legacy_decoded = MainchainSignature::decode(&mut legacy_encoded.as_slice())
			.expect("Encoded legacy should decode to current type");

		assert_eq!(legacy_decoded.0, [10; MAINCHAIN_SIGNATURE_LEN]);

		let current_encoded = MainchainSignature([9; MAINCHAIN_SIGNATURE_LEN]).encode();

		let current_decoded = LegacyMCSignature::decode(&mut current_encoded.as_slice())
			.expect("Encoded current should decode to legacy");

		assert_eq!(current_decoded.0, vec![9; 64]);
	}

	#[test]
	fn cross_chain_signature_verify_works() {
		let signature =	CrossChainSignature(
			hex!("d1e02e4a5484c3b7202ce6b844577048e7578dc62901cf8f51e6d74bbd3adb091688feacedd8343d0b04a0f5862b2e06148934a75e678e42051fde5431eca33d").to_vec()
		);
		let pubkey = CrossChainPublicKey(
			hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec(),
		);
		let signed_data = hex!(
			"84020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a16c68747470733a2f2f636f6f6c2e73747566662f73706f2e6a736f6e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
		);

		assert!(signature.verify(&pubkey, &signed_data).is_ok())
	}
}

#[derive(
	Clone,
	PartialEq,
	Eq,
	Ord,
	PartialOrd,
	TypeInfo,
	MaxEncodedLen,
	Encode,
	Decode,
	DecodeWithMemTracking,
)]
/// Represents a Cardano ADA delegator
pub enum DelegatorKey {
	/// Represents a staking address that is controlled by a user delegator
	StakeKeyHash([u8; 28]),
	/// Represents a staking address that is locked by a Plutus script
	ScriptKeyHash {
		/// Raw stake address hash
		hash_raw: [u8; 28],
		/// Hash of the Plutus script controlling the staking address
		script_hash: [u8; 28],
	},
}

impl alloc::fmt::Debug for DelegatorKey {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let s = match self {
			Self::ScriptKeyHash { hash_raw, script_hash } => alloc::format!(
				"ScriptKeyHash{{ hash_raw: {}, script_hash: {} }}",
				hex::encode(hash_raw),
				hex::encode(script_hash)
			),
			Self::StakeKeyHash(hash) => alloc::format!("StakeKeyHash({})", hex::encode(hash)),
		};

		f.write_str(&s)
	}
}

/// Amount of Lovelace staked by a Cardano delegator to a single stake pool
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct DelegatorStakeAmount(pub u64);

impl<T: Into<u64>> From<T> for DelegatorStakeAmount {
	fn from(value: T) -> Self {
		Self(value.into())
	}
}

/// A mapping between Cardano SPOs and the information about ADA delegation of their stake pools
///
/// This mapping can be used to calculate relative share of the total delegation for the
/// purpose of weighing during block producer selection.
#[derive(Debug, Clone, Default)]
pub struct StakeDistribution(pub BTreeMap<MainchainKeyHash, PoolDelegation>);

/// ADA delegation data for a single Cardano SPO
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoolDelegation {
	/// Total amount delegated to the stake pool
	pub total_stake: StakeDelegation,
	/// Delegated amount for each delegator of the stake pool
	pub delegators: BTreeMap<DelegatorKey, DelegatorStakeAmount>,
}
