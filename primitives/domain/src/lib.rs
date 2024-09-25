//! Types/Structs/Functions meant to be used by other modules / our Business Logic (i.e the Sidechain's node / runtime in our case)

#![cfg_attr(not(feature = "std"), no_std)]

pub mod byte_string;
pub mod crypto;

extern crate alloc;
extern crate core;
extern crate num_derive;

pub use alloc::vec::Vec;
use alloc::{str::FromStr, string::ToString, vec};
use byte_string_derive::byte_string;
use core::fmt::{Display, Formatter};
use crypto::blake2b;
use derive_more::{From, Into};
use num_derive::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use plutus::{Datum, ToDatum};
use plutus_datum_derive::*;
use scale_info::TypeInfo;
use sp_core::{bounded::BoundedVec, ed25519, sr25519, ConstU32};
#[cfg(feature = "serde")]
use {
	derive_more::FromStr,
	serde::{Deserialize, Deserializer, Serialize, Serializer},
	sp_core::bytes::from_hex,
};

/// A main chain epoch number. In range [0, 2^31-1].
#[derive(
	Default, Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, Hash, TypeInfo, PartialOrd,
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
#[derive(Default, Clone, Copy, Debug, Encode, Decode, TypeInfo, ToDatum, PartialEq, Eq)]
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

#[cfg(feature = "serde")]
impl FromStr for MainchainAddress {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let bytes: Vec<u8> = s.as_bytes().to_vec();
		let bounded = BoundedVec::try_from(bytes).map_err(|_| "Invalid length")?;
		Ok(MainchainAddress(bounded))
	}
}

#[cfg(feature = "std")]
impl ToString for MainchainAddress {
	fn to_string(&self) -> String {
		String::from_utf8(self.0.to_vec())
			.expect("MainchainAddressString is always properly encoded UTF-8")
	}
}

/// Cardano Policy Id is a 224 bits blake2b hash.
const POLICY_ID_LEN: usize = 28;
/// Cardano Policy Id
#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen, Hash)]
#[byte_string(debug, decode_hex, hex_serialize, hex_deserialize)]
#[cfg_attr(feature = "std", byte_string(to_hex_string))]
pub struct PolicyId(pub [u8; POLICY_ID_LEN]);

pub const MAX_ASSET_NAME_LEN: u32 = 32;

#[derive(Clone, Default, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen)]
#[byte_string(debug, hex_serialize, hex_deserialize, decode_hex)]
pub struct AssetName(pub BoundedVec<u8, ConstU32<MAX_ASSET_NAME_LEN>>);

const MAINCHAIN_PUBLIC_KEY_LEN: usize = 32;

#[derive(Clone, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen, Hash)]
#[byte_string(debug, hex_serialize, hex_deserialize)]
pub struct MainchainPublicKey(pub [u8; MAINCHAIN_PUBLIC_KEY_LEN]);

#[cfg(feature = "serde")]
impl FromStr for MainchainPublicKey {
	type Err = &'static str;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let bytes_vec =
			from_hex(s).map_err(|_| "Mainchain Public Key must be a valid hex string")?;
		bytes_vec.try_into()
	}
}

impl TryFrom<Vec<u8>> for MainchainPublicKey {
	type Error = &'static str;

	fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
		<[u8; 32]>::try_from(value)
			.map_err(|_| "Mainchain public key must be 32 bytes long")
			.map(MainchainPublicKey)
	}
}

pub const MAINCHAIN_ADDRESS_HASH_LEN: usize = 28;

/// Some hash of MainchainAddress, 28 bytes. Presumably blake2b_224.
/// Way to get it: cardano-cli address key-hash --payment-verification-key-file <path to vkey>
#[derive(
	Clone, Copy, Decode, Default, Eq, Encode, Hash, MaxEncodedLen, PartialEq, ToDatum, TypeInfo,
)]
#[byte_string(debug)]
#[cfg_attr(feature = "std", byte_string(to_hex_string, decode_hex))]
#[cfg_attr(feature = "serde", byte_string(hex_serialize, hex_deserialize))]
pub struct MainchainAddressHash(pub [u8; MAINCHAIN_ADDRESS_HASH_LEN]);

impl Display for MainchainAddressHash {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let hash = sp_core::hexdisplay::HexDisplay::from(&self.0);
		write!(f, "0x{}", hash)
	}
}

impl MainchainAddressHash {
	pub fn from_vkey(vkey: [u8; 32]) -> Self {
		Self(blake2b(&vkey))
	}
}

#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
#[byte_string(debug, hex_serialize)]
pub struct MainchainSignature(pub Vec<u8>);

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

#[derive(Clone, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo)]
#[byte_string(debug, hex_serialize, hex_deserialize, as_ref)]
pub struct SidechainPublicKey(pub Vec<u8>);

#[derive(Clone, Encode, Decode, TypeInfo, PartialEq, Eq)]
#[byte_string(debug, hex_serialize, hex_deserialize)]
pub struct SidechainSignature(pub Vec<u8>);

#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct CrossChainPublicKey(pub Vec<u8>);

#[derive(Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct CrossChainSignature(pub Vec<u8>);

#[derive(Default, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[byte_string(debug, hex_serialize)]
pub struct EpochNonce(pub Vec<u8>);

/// Identifies UTxO by transaction hash, and the index of output.
#[derive(
	Default, Debug, Copy, Clone, PartialEq, Eq, Encode, Decode, ToDatum, TypeInfo, MaxEncodedLen,
)]
pub struct UtxoId {
	pub tx_hash: McTxHash,
	pub index: UtxoIndex,
}

impl UtxoId {
	pub fn new(hash: [u8; TX_HASH_SIZE], index: u16) -> UtxoId {
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
	pub consumed_input: UtxoId,
	pub sidechain_signature: SidechainSignature,
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
	pub mainchain_pub_key: MainchainPublicKey,
	/// **List of Registrations** done by the **Authority Candidate**
	pub registrations: Vec<RegistrationData>,
	pub stake_delegation: Option<StakeDelegation>,
}

impl CandidateRegistrations {
	pub fn new(
		mainchain_pub_key: MainchainPublicKey,
		stake_delegation: Option<StakeDelegation>,
		registrations: Vec<RegistrationData>,
	) -> Self {
		Self { mainchain_pub_key, registrations, stake_delegation }
	}

	pub fn mainchain_pub_key(&self) -> &MainchainPublicKey {
		&self.mainchain_pub_key
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
/// https://github.com/input-output-hk/partner-chains-smart-contracts/blob/5b19d25a95c3ab49ae0e4c6ce0ec3376f13b3766/docs/Specification.md#L554-L561
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

#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[byte_string(debug, hex_serialize, hex_deserialize)]
pub struct AuraPublicKey(pub Vec<u8>);
impl AuraPublicKey {
	pub fn try_into_sr25519(&self) -> Option<sr25519::Public> {
		Some(sr25519::Public::from_raw(self.0.clone().try_into().ok()?))
	}
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
#[byte_string(debug, hex_serialize, hex_deserialize)]
pub struct GrandpaPublicKey(pub Vec<u8>);
impl GrandpaPublicKey {
	pub fn try_into_ed25519(&self) -> Option<ed25519::Public> {
		Some(ed25519::Public::from_raw(self.0.clone().try_into().ok()?))
	}
}

#[derive(Debug, Clone, PartialEq, Decode, Encode, MaxEncodedLen, TypeInfo, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct DParameter {
	pub num_permissioned_candidates: u16,
	pub num_registered_candidates: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, TypeInfo)]
pub struct PermissionedCandidateData {
	pub sidechain_public_key: SidechainPublicKey,
	pub aura_public_key: AuraPublicKey,
	pub grandpa_public_key: GrandpaPublicKey,
}

#[cfg(test)]
mod tests {
	use super::MainchainAddress;
	use core::str::FromStr;

	#[test]
	fn main_chain_address_string_serialize_deserialize_round_trip() {
		use super::MainchainAddress;
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
}
