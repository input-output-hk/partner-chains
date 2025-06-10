//! # Key Parameter Types for CLI Commands
//!
//! This module provides wrapper types for various cryptographic keys used in
//! Partner Chain operations. These types handle secure parsing, validation,
//! and conversion of key material from command-line string inputs into the
//! appropriate cryptographic primitives.
//!
//! ## Key Type Categories
//!
//! - **Sidechain Keys**: ECDSA keys for Partner Chain validator operations
//! - **Mainchain Keys**: Ed25519 keys for Cardano stake pool operations  
//! - **Cross-Chain Keys**: ECDSA keys for cross-chain bridge operations
//! - **Stake Keys**: Ed25519 keys for general Cardano staking operations
//!
//! ## Security Considerations
//!
//! All key parameter types provide secure parsing that validates key material
//! format and cryptographic validity. Input validation prevents malformed keys
//! from propagating through the system and ensures only valid cryptographic
//! material is used for signature generation.

use sidechain_domain::*;
use std::convert::Infallible;
use std::fmt::Display;
use std::io;
use std::io::ErrorKind;
use std::str::FromStr;

/// Wrapper type for ECDSA private keys used in Partner Chain validator operations.
///
/// This struct encapsulates a secp256k1 private key and provides secure parsing
/// from string representations. The key is used for signing operations on the
/// Partner Chain side of cross-chain transactions and validator registration.
///
/// ## Key Format
///
/// Accepts hexadecimal string inputs with optional "0x" prefix. The underlying
/// key material must be a valid 32-byte secp256k1 private key.
///
/// ## Usage
///
/// This type is primarily used in registration signature commands where validators
/// must prove their identity on the Partner Chain network through ECDSA signatures.
#[derive(Clone, Debug)]
pub struct SidechainSigningKeyParam(pub secp256k1::SecretKey);

impl SidechainSigningKeyParam {
	/// Derives the corresponding ECDSA public key from this private key.
	///
	/// This method uses the global secp256k1 context to perform the key derivation,
	/// ensuring consistent public key generation across the system.
	///
	/// # Returns
	/// The secp256k1 public key corresponding to this private key
	pub fn to_pub_key(&self) -> secp256k1::PublicKey {
		secp256k1::PublicKey::from_secret_key_global(&self.0)
	}
}

impl FromStr for SidechainSigningKeyParam {
	type Err = secp256k1::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		let pair = secp256k1::SecretKey::from_str(trimmed)?;
		Ok(SidechainSigningKeyParam(pair))
	}
}

/// Wrapper type for ECDSA public keys used in Partner Chain operations.
///
/// This struct encapsulates a Partner Chain public key and provides parsing
/// and display functionality for ECDSA public keys in hexadecimal format.
///
/// ## Key Format
///
/// The public key is stored as a byte vector containing the compressed secp256k1
/// public key representation (33 bytes starting with 0x02 or 0x03).
#[derive(Clone, Debug)]
pub struct SidechainPublicKeyParam(pub SidechainPublicKey);

impl Display for SidechainPublicKeyParam {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "0x{}", hex::encode(&self.0.0))
	}
}

impl FromStr for SidechainPublicKeyParam {
	type Err = secp256k1::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		let pk = secp256k1::PublicKey::from_str(trimmed)?;
		Ok(SidechainPublicKeyParam(SidechainPublicKey(pk.serialize().to_vec())))
	}
}

/// Generic wrapper for plain string public key parameters.
///
/// This type provides a simple container for public key strings that don't
/// require cryptographic validation, useful for scenarios where the key
/// format is flexible or application-specific.
#[derive(Clone, Debug)]
pub struct PlainPublicKeyParam(pub String);

impl Display for PlainPublicKeyParam {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl FromStr for PlainPublicKeyParam {
	type Err = Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(PlainPublicKeyParam(s.to_string()))
	}
}

/// Error types that can occur during Ed25519 signing key parsing.
#[derive(Debug, thiserror::Error)]
pub enum Ed25519SigningKeyError {
	/// Hexadecimal decoding error
	#[error("{0}")]
	HexError(#[from] hex::FromHexError),
	/// Ed25519 key validation error
	#[error("{0}")]
	Ed25519Error(#[from] ed25519_zebra::Error),
}

impl From<Ed25519SigningKeyError> for io::Error {
	fn from(value: Ed25519SigningKeyError) -> Self {
		io::Error::new(ErrorKind::InvalidInput, value)
	}
}

/// Parses a hexadecimal string into an ed25519-zebra signing key.
///
/// This internal function provides common parsing logic for Ed25519 keys
/// used across multiple key parameter types. It handles hex decoding and
/// cryptographic validation in a consistent manner.
///
/// # Arguments
/// * `s` - Hexadecimal string representation of the Ed25519 private key
///
/// # Returns
/// * `Ok(ed25519_zebra::SigningKey)` - Successfully parsed and validated key
/// * `Err(Ed25519SigningKeyError)` - Parsing or validation failure
///
/// # Key Format
/// Expects a 32-byte (64 character) hexadecimal string with optional "0x" prefix.
pub(crate) fn parse_zebra_signing_key(
	s: &str,
) -> Result<ed25519_zebra::SigningKey, Ed25519SigningKeyError> {
	let trimmed = s.trim_start_matches("0x");
	Ok(ed25519_zebra::SigningKey::try_from(hex::decode(trimmed)?.as_slice())?)
}

/// Wrapper type for Ed25519 private keys used in Cardano stake pool operations.
///
/// This struct encapsulates an Ed25519 private key specifically used for
/// Cardano stake pool operator signing operations. The key is used in
/// validator registration processes to prove stake pool operator authority.
///
/// ## Key Format
///
/// Accepts 32-byte Ed25519 private keys in hexadecimal format with optional
/// "0x" prefix. The key undergoes cryptographic validation to ensure it
/// represents a valid Ed25519 private key.
///
/// ## Usage
///
/// This type is used in registration signature commands where stake pool
/// operators must prove their authority to register validators on Partner Chains.
#[derive(Clone, Debug)]
pub struct StakePoolSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakePoolSigningKeyParam {
	type Err = Ed25519SigningKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(parse_zebra_signing_key(s)?))
	}
}

impl From<[u8; 32]> for StakePoolSigningKeyParam {
	/// Creates a stake pool signing key parameter from a 32-byte array.
	///
	/// This method provides direct construction from raw key bytes,
	/// useful for programmatic key generation or testing scenarios.
	///
	/// # Arguments
	/// * `key` - 32-byte array containing the Ed25519 private key
	///
	/// # Returns
	/// A new `StakePoolSigningKeyParam` instance
	fn from(key: [u8; 32]) -> Self {
		Self(ed25519_zebra::SigningKey::from(key))
	}
}

impl StakePoolSigningKeyParam {
	/// Returns the `StakePoolPublicKey` corresponding to this private key
	pub fn vkey(&self) -> StakePoolPublicKey {
		StakePoolPublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}

/// Wrapper type for Ed25519 private keys used in general Cardano staking operations.
///
/// This struct encapsulates an Ed25519 private key used for Cardano stake
/// address operations, particularly in address association commands where
/// stake key holders prove their authority to link addresses.
///
/// ## Key Format
///
/// Accepts 32-byte Ed25519 private keys in hexadecimal format with optional
/// "0x" prefix. Key validation ensures cryptographic correctness.
///
/// ## Usage
///
/// This type is primarily used in address association commands where Cardano
/// stake key holders authorize the linking of their stake address with a
/// Partner Chain address.
#[derive(Clone, Debug)]
pub struct StakeSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakeSigningKeyParam {
	type Err = Ed25519SigningKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(parse_zebra_signing_key(s)?))
	}
}

impl StakeSigningKeyParam {
	/// Derives the corresponding Ed25519 public key for stake operations.
	///
	/// This method generates the verification key that corresponds to this
	/// signing key, which is used in stake address operations and verification.
	///
	/// # Returns
	/// The `StakePublicKey` corresponding to this private key
	pub fn vkey(&self) -> StakePublicKey {
		StakePublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}

/// Wrapper type for ECDSA private keys used in cross-chain bridge operations.
///
/// This struct encapsulates a secp256k1 private key (via k256 implementation)
/// used for cross-chain operations between Cardano and Partner Chain networks.
/// The key enables block producers to prove their identity across both chains.
///
/// ## Key Format
///
/// Accepts 32-byte secp256k1 private keys in hexadecimal format. The key
/// undergoes validation to ensure it represents a valid ECDSA private key.
///
/// ## Cross-Chain Integration
///
/// This key type is specifically designed for operations that span both
/// Cardano and Partner Chain networks, enabling secure cross-chain identity
/// verification and metadata signing for block producers.
#[derive(Clone, Debug)]
pub struct CrossChainSigningKeyParam(pub k256::SecretKey);

impl FromStr for CrossChainSigningKeyParam {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(k256::SecretKey::from_slice(&hex::decode(s)?)?))
	}
}

impl CrossChainSigningKeyParam {
	/// Derives the corresponding ECDSA public key for cross-chain operations.
	///
	/// This method generates the verification key that corresponds to this
	/// signing key, formatted as compressed SEC1 bytes for cross-chain compatibility.
	///
	/// # Returns
	/// The `CrossChainPublicKey` corresponding to this private key, containing
	/// the compressed SEC1 representation of the ECDSA public key
	pub fn vkey(&self) -> CrossChainPublicKey {
		CrossChainPublicKey(self.0.public_key().to_sec1_bytes().to_vec())
	}
}
