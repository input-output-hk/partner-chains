//! Types/Structs/Functions meant to be used by other modules / our Business Logic (i.e the Sidechain's node / runtime in our case)

#![cfg_attr(not(feature = "std"), no_std)]

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
use parity_scale_codec::{Decode, Encode, MaxEncodedLen, WrapperTypeEncode};
use plutus_datum_derive::*;
use scale_info::TypeInfo;
use sp_core::{bounded::BoundedVec, ecdsa, ed25519, sr25519, ConstU32};
#[cfg(feature = "serde")]
use {
	derive_more::FromStr,
	serde::{Deserialize, Deserializer, Serialize, Serializer},
};

/// The number of main chain epochs back a Partner Chain queries for committee selection inputs.
/// This offset is necessary to ensure that data is present and stable.
const DATA_MC_EPOCH_OFFSET: u32 = 2;

pub fn offset_data_epoch(epoch: &McEpochNumber) -> Result<McEpochNumber, u32> {
	Ok(McEpochNumber(epoch.0.checked_sub(DATA_MC_EPOCH_OFFSET).ok_or(DATA_MC_EPOCH_OFFSET)?))
}

/// A main chain epoch number. In range [0, 2^31-1].
#[derive(
	Default, Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, Hash, TypeInfo, Ord, PartialOrd,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
pub struct McEpochNumber(pub u32);

impl McEpochNumber {
	pub fn next(&self) -> Self {
		Self(&self.0 + 1)
	}
}

impl Display for McEpochNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u32::fmt(&self.0, f)
	}
}
/// Amount of Lovelace (which is a fraction of 1 ADA) staked/locked on Cardano
#[derive(
	Default, Clone, Copy, Debug, Encode, Decode, TypeInfo, ToDatum, PartialEq, Eq, PartialOrd, Ord,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct StakeDelegation(pub u64);

impl StakeDelegation {
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
	TypeInfo,
	ToDatum,
	PartialEq,
	Eq,
	From,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct NativeTokenAmount(pub u128);

impl Display for NativeTokenAmount {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u128::fmt(&self.0, f)
	}
}

/// A main chain block number. In range [0, 2^31-1].
#[derive(
	Default,
	Clone,
	Copy,
	Debug,
	Encode,
	Decode,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	scale_info::TypeInfo,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
pub struct McBlockNumber(pub u32);

impl Display for McBlockNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u32::fmt(&self.0, f)
	}
}

#[derive(
	Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Encode, Decode, TypeInfo, Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
pub struct McSlotNumber(pub u64);

impl Display for McSlotNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u64::fmt(&self.0, f)
	}
}

#[derive(
	Default, Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, TypeInfo, Hash, MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, FromStr))]
pub struct ScSlotNumber(pub u64);

#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MainchainBlock {
	pub number: McBlockNumber,
	pub hash: McBlockHash,
	pub epoch: McEpochNumber,
	pub slot: McSlotNumber,
	pub timestamp: u64, // seconds since UNIX_EPOCH
}

/// An index of transaction in a block. In range [0, 2^31-1].
#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	PartialOrd,
	Ord,
	ToDatum,
	TypeInfo,
	MaxEncodedLen,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

const MAX_MAINCHAIN_ADDRESS_BYTES: u32 = 120;

/// Wraps UTF-8 bytes of Mainchain Address in bech32 format.
/// Example: utf-8 bytes of "addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc" are
/// "0x616464725f7465737431777a35716337666b327061743030353877347a77766b77333579747074656a336e7563336a65326b6774616e356471337274347363"
#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
#[byte_string(debug)]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
pub struct MainchainAddress(BoundedVec<u8, ConstU32<MAX_MAINCHAIN_ADDRESS_BYTES>>);

impl MainchainAddress {
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

/// Cardano Policy Id is a 224 bits blake2b hash.
const POLICY_ID_LEN: usize = 28;
/// Cardano Policy Id
#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen, Hash)]
#[byte_string(debug, decode_hex, hex_serialize, hex_deserialize)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
pub struct PolicyId(pub [u8; POLICY_ID_LEN]);

pub type ScriptHash = PolicyId;

pub const MAX_ASSET_NAME_LEN: u32 = 32;

#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct AssetName(pub BoundedVec<u8, ConstU32<MAX_ASSET_NAME_LEN>>);

impl AssetName {
	pub fn empty() -> Self {
		Self(BoundedVec::new())
	}
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetId {
	pub policy_id: PolicyId,
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

const STAKE_POOL_PUBLIC_KEY_LEN: usize = 32;

#[derive(
	Clone, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen, Hash, Ord, PartialOrd,
)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct StakePoolPublicKey(pub [u8; STAKE_POOL_PUBLIC_KEY_LEN]);

impl StakePoolPublicKey {
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

const STAKE_PUBLIC_KEY_LEN: usize = 32;

#[derive(Clone, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen, Hash)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct StakePublicKey(pub [u8; STAKE_PUBLIC_KEY_LEN]);

impl StakePublicKey {
	pub fn hash(&self) -> MainchainKeyHash {
		MainchainKeyHash(blake2b(&self.0))
	}
}

const MAINCHAIN_KEY_HASH_LEN: usize = 28;

/// blake2b_224 hash of a Cardano Verification (Public) Key.
/// It can be a hash of Payment Verification, Payment Extended Verification, Stake Pool Verification Key or Staking Verification Key.
#[derive(
	Clone,
	Copy,
	Decode,
	Default,
	Encode,
	Hash,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	ToDatum,
	TypeInfo,
)]
#[byte_string(debug)]
#[cfg_attr(feature = "std", byte_string(to_hex_string, decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
pub struct MainchainKeyHash(pub [u8; MAINCHAIN_KEY_HASH_LEN]);

impl Display for MainchainKeyHash {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let hash = sp_core::hexdisplay::HexDisplay::from(&self.0);
		write!(f, "0x{}", hash)
	}
}

impl MainchainKeyHash {
	pub fn from_vkey(vkey: &[u8; 32]) -> Self {
		Self(blake2b(vkey))
	}
}

/// EDDSA signature, 64 bytes.
pub const MAINCHAIN_SIGNATURE_LEN: usize = 64;

#[derive(Clone, TypeInfo, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
#[byte_string(debug, hex_serialize, decode_hex)]
/// EDDSA signature
///
/// WARNING: This type needs to be backwards compatibile with a legacy schema wrapping `Vec<u8>`.
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
	pub fn verify(&self, public_key: &StakePoolPublicKey, signed_message: &[u8]) -> bool {
		let mainchain_signature = ed25519::Signature::from(self.0);

		sp_io::crypto::ed25519_verify(
			&mainchain_signature,
			signed_message,
			&ed25519::Public::from(public_key.0),
		)
	}
}

/// EDDSA signature, 64 bytes.
pub const STAKE_KEY_SIGNATURE_LEN: usize = 64;

#[derive(Clone, Encode, Decode, PartialEq, Eq, Hash, TypeInfo)]
#[byte_string(debug, hex_serialize, decode_hex)]
/// EDDSA signature made with Stake Signing Key
pub struct StakeKeySignature(pub [u8; STAKE_KEY_SIGNATURE_LEN]);

impl From<[u8; STAKE_KEY_SIGNATURE_LEN]> for StakeKeySignature {
	fn from(raw: [u8; STAKE_KEY_SIGNATURE_LEN]) -> Self {
		Self(raw)
	}
}

impl StakeKeySignature {
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
pub struct ScEpochNumber(pub u64);

impl Display for ScEpochNumber {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		u64::fmt(&self.0, f)
	}
}

impl ScEpochNumber {
	pub fn next(&self) -> Self {
		Self(self.0 + 1)
	}
	pub fn prev(&self) -> Option<Self> {
		self.0.checked_sub(1).map(Self)
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, PartialOrd, Ord, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex, as_ref)]
pub struct SidechainPublicKey(pub Vec<u8>);

impl From<ecdsa::Public> for SidechainPublicKey {
	fn from(value: ecdsa::Public) -> Self {
		Self(value.0.to_vec())
	}
}

/// CBOR bytes of Cardano Transaction.
#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct TransactionCbor(pub Vec<u8>);

/// CBOR bytes of Cardano VKeyWitness.
#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct VKeyWitnessCbor(pub Vec<u8>);

#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct SidechainSignature(pub Vec<u8>);

#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct CrossChainPublicKey(pub Vec<u8>);

impl CrossChainPublicKey {
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

const CROSS_CHAIN_KEY_HASH_LEN: usize = 28;

#[derive(
	Clone,
	Copy,
	Decode,
	Default,
	Encode,
	Hash,
	MaxEncodedLen,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	ToDatum,
	TypeInfo,
)]
#[byte_string(debug, to_hex_string)]
#[cfg_attr(feature = "std", byte_string(decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
pub struct CrossChainKeyHash(pub [u8; CROSS_CHAIN_KEY_HASH_LEN]);

impl Display for CrossChainKeyHash {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		f.write_str(&self.to_hex_string())
	}
}

#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct CrossChainSignature(pub Vec<u8>);

impl CrossChainSignature {
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

#[derive(Default, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct EpochNonce(pub Vec<u8>);

/// Identifies UTxO by transaction hash, and the index of output.
#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	ToDatum,
	TypeInfo,
	MaxEncodedLen,
	Hash,
)]
pub struct UtxoId {
	pub tx_hash: McTxHash,
	pub index: UtxoIndex,
}

impl UtxoId {
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

/// An index of output of a transaction. In range [0, 2^15-1].
#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	PartialOrd,
	Ord,
	ToDatum,
	TypeInfo,
	MaxEncodedLen,
	Hash,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

pub const TX_HASH_SIZE: usize = 32;

#[derive(
	Default, Copy, Clone, Hash, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen,
)]
#[byte_string(debug, from_bytes, decode_hex, hex_serialize, hex_deserialize)]
#[constructor_datum]
pub struct McTxHash(pub [u8; TX_HASH_SIZE]);

impl TryFrom<Vec<u8>> for McTxHash {
	type Error = &'static str;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		<[u8; 32]>::try_from(value)
			.map_err(|_| "McTxHash must be 32 bytes long")
			.map(McTxHash)
	}
}

#[derive(Default, Clone, Decode, Encode, PartialEq, Eq, TypeInfo, ToDatum, MaxEncodedLen, Hash)]
#[byte_string(debug, decode_hex, hex_serialize, hex_deserialize)]
pub struct McBlockHash(pub [u8; 32]);

impl Display for McBlockHash {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let hash = sp_core::hexdisplay::HexDisplay::from(&self.0);
		write!(f, "{}", hash)
	}
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UtxoInfo {
	pub utxo_id: UtxoId,
	pub epoch_number: McEpochNumber,
	pub block_number: McBlockNumber,
	pub slot_number: McSlotNumber,
	pub tx_index_within_block: McTxIndexInBlock,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtxoInfoOrderingKey {
	pub block_number: McBlockNumber,
	pub tx_index_within_block: McTxIndexInBlock,
	pub utxo_id_index: UtxoIndex,
}

impl UtxoInfo {
	pub fn ordering_key(&self) -> UtxoInfoOrderingKey {
		UtxoInfoOrderingKey {
			block_number: self.block_number,
			tx_index_within_block: self.tx_index_within_block,
			utxo_id_index: self.utxo_id.index,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum NetworkType {
	Mainnet,
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

#[derive(Default, Clone, PartialEq, Eq, Encode, Decode)]
#[byte_string(debug, hex_serialize, hex_deserialize)]
pub struct CommitteeHash(pub Vec<u8>);

/// UTxO Output of a Registration Transaction on Cardano
///
/// Note: A Registration Transaction is called by a user on Cardano to register themselves as a Sidechain Authority Candidate
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct RegistrationData {
	/// UTXO that is an input parameter to the registration transaction
	pub registration_utxo: UtxoId,
	pub sidechain_signature: SidechainSignature,
	/// Stake Pool key signature
	pub mainchain_signature: MainchainSignature,
	pub cross_chain_signature: CrossChainSignature,
	pub sidechain_pub_key: SidechainPublicKey,
	pub cross_chain_pub_key: CrossChainPublicKey,
	pub utxo_info: UtxoInfo,
	pub tx_inputs: Vec<UtxoId>,
	pub aura_pub_key: AuraPublicKey,
	pub grandpa_pub_key: GrandpaPublicKey,
}

/// Information about an Authority Candidate's Registrations at some block.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct CandidateRegistrations {
	pub stake_pool_public_key: StakePoolPublicKey,
	/// **List of Registrations** done by the **Authority Candidate**
	pub registrations: Vec<RegistrationData>,
	pub stake_delegation: Option<StakeDelegation>,
}

impl CandidateRegistrations {
	pub fn new(
		stake_pool_public_key: StakePoolPublicKey,
		stake_delegation: Option<StakeDelegation>,
		registrations: Vec<RegistrationData>,
	) -> Self {
		Self { stake_pool_public_key, registrations, stake_delegation }
	}

	pub fn mainchain_pub_key(&self) -> &StakePoolPublicKey {
		&self.stake_pool_public_key
	}

	pub fn registrations(&self) -> &[RegistrationData] {
		&self.registrations
	}
}

/// Hash of concatenated private keys. It should match:
/// -- | Invariant: 'ATMSPlainAggregatePubKey' is the sorted concatenated hash of
/// -- sidechain public keys. More precisely,
/// -- @
/// -- committeePubKeys = sort([key1, key2, ..., keyN])
/// -- committeePubKeysHash = blake2b_256(concat(committeePubKeys))
/// -- keyi - 33 bytes compressed ECDSA public key of a committee member
/// -- @
/// newtype ATMSPlainAggregatePubKey = ATMSPlainAggregatePubKey ByteString
/// <https://github.com/input-output-hk/partner-chains-smart-contracts/blob/5b19d25a95c3ab49ae0e4c6ce0ec3376f13b3766/docs/Specification.md#L554-L561>
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq, ToDatum, TypeInfo)]
pub struct ATMSPlainAggregatePubKey(pub [u8; 32]);

impl ATMSPlainAggregatePubKey {
	pub fn new(mut keys: Vec<SidechainPublicKey>) -> Self {
		keys.sort_by(|a, b| a.0.cmp(&b.0));
		let concatenated: Vec<u8> = keys.into_iter().flat_map(|k| k.0).collect();
		let hashed = sp_crypto_hashing::blake2_256(concatenated.as_slice());
		Self(hashed)
	}
}

pub const VALIDATOR_HASH_LEN: usize = 28;

#[derive(Clone, Decode, Default, Eq, Encode, MaxEncodedLen, PartialEq, TypeInfo, ToDatum)]
#[byte_string(debug, decode_hex, hex_serialize, hex_deserialize)]
pub struct ValidatorHash(pub [u8; VALIDATOR_HASH_LEN]);

#[derive(Debug, Clone, TypeInfo, Decode, Encode, Eq, PartialEq, ToDatum)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SidechainPublicKeysSorted(Vec<SidechainPublicKey>);

impl SidechainPublicKeysSorted {
	pub fn new(mut keys: Vec<SidechainPublicKey>) -> Self {
		keys.sort_by(|a, b| a.0.cmp(&b.0));
		Self(keys)
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, PartialOrd, Ord, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct AuraPublicKey(pub Vec<u8>);
impl AuraPublicKey {
	pub fn try_into_sr25519(&self) -> Option<sr25519::Public> {
		Some(sr25519::Public::from_raw(self.0.clone().try_into().ok()?))
	}
}

impl From<sr25519::Public> for AuraPublicKey {
	fn from(value: sr25519::Public) -> Self {
		Self(value.0.to_vec())
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, PartialOrd, Ord, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct GrandpaPublicKey(pub Vec<u8>);
impl GrandpaPublicKey {
	pub fn try_into_ed25519(&self) -> Option<ed25519::Public> {
		Some(ed25519::Public::from_raw(self.0.clone().try_into().ok()?))
	}
}

impl From<ed25519::Public> for GrandpaPublicKey {
	fn from(value: ed25519::Public) -> Self {
		Self(value.0.to_vec())
	}
}

#[derive(Debug, Clone, PartialEq, Decode, Encode, MaxEncodedLen, TypeInfo, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DParameter {
	pub num_permissioned_candidates: u16,
	pub num_registered_candidates: u16,
}

impl DParameter {
	pub fn new(num_permissioned_candidates: u16, num_registered_candidates: u16) -> Self {
		Self { num_permissioned_candidates, num_registered_candidates }
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, TypeInfo, PartialOrd, Ord, Hash)]
pub struct PermissionedCandidateData {
	pub sidechain_public_key: SidechainPublicKey,
	pub aura_public_key: AuraPublicKey,
	pub grandpa_public_key: GrandpaPublicKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CandidateRegistration {
	pub stake_ownership: AdaBasedStaking,
	pub partner_chain_pub_key: SidechainPublicKey,
	pub partner_chain_signature: SidechainSignature,
	pub own_pkh: MainchainKeyHash,
	pub registration_utxo: UtxoId,
	pub aura_pub_key: AuraPublicKey,
	pub grandpa_pub_key: GrandpaPublicKey,
}

impl CandidateRegistration {
	pub fn matches_keys(&self, other: &Self) -> bool {
		self.stake_ownership == other.stake_ownership
			&& self.partner_chain_pub_key == other.partner_chain_pub_key
			&& self.partner_chain_signature == other.partner_chain_signature
			&& self.aura_pub_key == other.aura_pub_key
			&& self.grandpa_pub_key == other.grandpa_pub_key
	}
}

/// AdaBasedStaking is a variant of Plutus type StakeOwnership.
/// The other variant, TokenBasedStaking, is not supported
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AdaBasedStaking {
	pub pub_key: StakePoolPublicKey,
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
		assert_eq!(serialized, serde_json::json!("0x616464725f7465737431777a35716337666b327061743030353877347a77766b77333579747074656a336e7563336a65326b6774616e356471337274347363"));
		let deserialized = serde_json::from_value(serialized).unwrap();
		assert_eq!(address, deserialized);
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
		#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq, Hash)]
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
		let signed_data = hex!("84020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a16c68747470733a2f2f636f6f6c2e73747566662f73706f2e6a736f6e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000");

		assert!(signature.verify(&pubkey, &signed_data).is_ok())
	}
}

#[derive(Clone, PartialEq, Eq, Ord, PartialOrd, TypeInfo, MaxEncodedLen, Encode, Decode)]
pub enum DelegatorKey {
	StakeKeyHash([u8; 28]),
	ScriptKeyHash { hash_raw: [u8; 28], script_hash: [u8; 28] },
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

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct DelegatorStakeAmount(pub u64);

impl<T: Into<u64>> From<T> for DelegatorStakeAmount {
	fn from(value: T) -> Self {
		Self(value.into())
	}
}

#[derive(Debug, Clone, Default)]
pub struct StakeDistribution(pub BTreeMap<MainchainKeyHash, PoolDelegation>);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PoolDelegation {
	pub total_stake: StakeDelegation,
	pub delegators: BTreeMap<DelegatorKey, DelegatorStakeAmount>,
}
