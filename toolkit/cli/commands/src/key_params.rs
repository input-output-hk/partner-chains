//! # Cryptographic Key Parameters
//!
//! Provides parameter types for various cryptographic keys used in Partner Chains operations.
//! This module defines wrapper types for different key algorithms and formats,
//! enabling secure key handling and CLI parameter parsing.
//!
//! ## Key Types Overview
//!
//! - **Sidechain Keys**: secp256k1 keys for Partner Chain operations
//! - **Stake Pool Keys**: Ed25519 keys for Cardano stake pool operations
//! - **Stake Keys**: Ed25519 keys for Cardano stake operations
//! - **Cross-Chain Keys**: ECDSA keys for cross-chain communication
//! - **Plain Keys**: String-based public key parameters
//!
//! ## Hex Format Support
//!
//! All key parameters support hex string input with optional "0x" prefix:
//!
//! ```bash
//! --signing-key d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84
//! --signing-key 0xd75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84
//! ```
//!
//! ## Key Conversions
//!
//! Each key parameter type provides methods to derive corresponding public keys:
//!
//! - `StakeSigningKeyParam::vkey()` → `StakePublicKey`
//! - `StakePoolSigningKeyParam::vkey()` → `StakePoolPublicKey`
//! - `CrossChainSigningKeyParam::vkey()` → `CrossChainPublicKey`
//! - `SidechainSigningKeyParam::to_pub_key()` → `secp256k1::PublicKey`

use sidechain_domain::*;
use std::convert::Infallible;
use std::fmt::Display;
use std::io;
use std::io::ErrorKind;
use std::str::FromStr;

/// Parameter type for sidechain signing keys.
///
/// Wraps a secp256k1 secret key for Partner Chain operations.
/// Used for signing sidechain transactions and validator registration.
///
/// ## Input Format
///
/// Accepts hex-encoded private key with optional "0x" prefix:
///
/// ```bash
/// --sidechain-signing-key 02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec
/// ```
#[derive(Clone, Debug)]
pub struct SidechainSigningKeyParam(pub secp256k1::SecretKey);

impl SidechainSigningKeyParam {
	/// Convert signing key to corresponding public key.
	///
	/// Derives the secp256k1 public key from the private key using global context.
	///
	/// ## Returns
	///
	/// `secp256k1::PublicKey` corresponding to this signing key.
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

/// Parameter type for sidechain public keys.
///
/// Wraps a `SidechainPublicKey` for CLI parameter parsing and display.
/// Used when public key input is required instead of private key.
///
/// ## Input Format
///
/// Accepts hex-encoded public key with optional "0x" prefix:
///
/// ```bash
/// --sidechain-public-key 02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec
/// ```
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

/// Parameter type for plain public key strings.
///
/// Simple wrapper for string-based public key parameters that don't require
/// specific cryptographic parsing or validation.
///
/// ## Input Format
///
/// Accepts any string as public key identifier:
///
/// ```bash
/// --public-key "any-string-identifier"
/// ```
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

/// Error type for Ed25519 signing key parsing operations.
///
/// Combines hex decoding errors and Ed25519 cryptographic errors
/// that can occur during key parameter parsing.
#[derive(Debug, thiserror::Error)]
pub enum Ed25519SigningKeyError {
	/// Hex decoding error when parsing key from string
	#[error("{0}")]
	HexError(#[from] hex::FromHexError),
	/// Ed25519 cryptographic error during key creation
	#[error("{0}")]
	Ed25519Error(#[from] ed25519_zebra::Error),
}

impl From<Ed25519SigningKeyError> for io::Error {
	fn from(value: Ed25519SigningKeyError) -> Self {
		io::Error::new(ErrorKind::InvalidInput, value)
	}
}

/// Parse Ed25519 signing key from hex string.
///
/// Converts hex-encoded string to `ed25519_zebra::SigningKey`, supporting
/// optional "0x" prefix removal. Used internally by Ed25519 key parameter types.
///
/// ## Parameters
///
/// - `s`: Hex-encoded signing key string with optional "0x" prefix
///
/// ## Returns
///
/// `Result<ed25519_zebra::SigningKey, Ed25519SigningKeyError>` containing:
/// - `Ok(SigningKey)`: Successfully parsed Ed25519 signing key
/// - `Err(Ed25519SigningKeyError)`: Parsing or key creation error
///
/// ## Errors
///
/// Returns error if:
/// - Hex decoding fails (invalid hex characters or length)
/// - Ed25519 key creation fails (invalid key bytes)
pub(crate) fn parse_zebra_signing_key(
	s: &str,
) -> Result<ed25519_zebra::SigningKey, Ed25519SigningKeyError> {
	let trimmed = s.trim_start_matches("0x");
	Ok(ed25519_zebra::SigningKey::try_from(hex::decode(trimmed)?.as_slice())?)
}

/// Parameter type for Cardano stake pool signing keys.
///
/// Wraps an Ed25519 signing key for Cardano stake pool operations.
/// Used for mainchain validator registration and stake pool management.
///
/// ## Input Format
///
/// Accepts hex-encoded Ed25519 private key with optional "0x" prefix:
///
/// ```bash
/// --mainchain-signing-key 2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c
/// ```
///
/// ## Key Requirements
///
/// The key must be a valid 32-byte Ed25519 private key. This corresponds to
/// Cardano stake pool signing keys used for mainchain operations.
#[derive(Clone, Debug)]
pub struct StakePoolSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakePoolSigningKeyParam {
	type Err = Ed25519SigningKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(parse_zebra_signing_key(s)?))
	}
}

impl From<[u8; 32]> for StakePoolSigningKeyParam {
	fn from(key: [u8; 32]) -> Self {
		Self(ed25519_zebra::SigningKey::from(key))
	}
}

impl StakePoolSigningKeyParam {
	/// Derive corresponding stake pool public key.
	///
	/// Generates the Ed25519 verification key (public key) from the signing key.
	/// Used for stake pool identification and signature verification.
	///
	/// ## Returns
	///
	/// `StakePoolPublicKey` corresponding to this signing key.
	pub fn vkey(&self) -> StakePoolPublicKey {
		StakePoolPublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}

/// Parameter type for Cardano stake signing keys.
///
/// Wraps an Ed25519 signing key for Cardano stake operations.
/// Used for address association and stake-related signatures.
///
/// ## Input Format
///
/// Accepts hex-encoded Ed25519 private key with optional "0x" prefix:
///
/// ```bash
/// --signing-key d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84
/// ```
///
/// ## Usage Context
///
/// Commonly used for:
/// - Address association signatures
/// - Stake delegation operations
/// - Cardano mainchain interactions
#[derive(Clone, Debug)]
pub struct StakeSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakeSigningKeyParam {
	type Err = Ed25519SigningKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(parse_zebra_signing_key(s)?))
	}
}

impl StakeSigningKeyParam {
	/// Derive corresponding stake public key.
	///
	/// Generates the Ed25519 verification key (public key) from the signing key.
	/// Used for stake identification and signature verification.
	///
	/// ## Returns
	///
	/// `StakePublicKey` corresponding to this signing key.
	pub fn vkey(&self) -> StakePublicKey {
		StakePublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}

/// Parameter type for cross-chain ECDSA signing keys.
///
/// Wraps a k256 ECDSA secret key for cross-chain operations.
/// Used for block producer metadata signing and cross-chain communication.
///
/// ## Input Format
///
/// Accepts hex-encoded ECDSA private key (no "0x" prefix required):
///
/// ```bash
/// --cross-chain-signing-key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
/// ```
///
/// ## Key Algorithm
///
/// Uses secp256k1 ECDSA (k256 implementation) for cross-chain signatures.
/// Compatible with Ethereum-style ECDSA keys and signatures.
#[derive(Clone, Debug)]
pub struct CrossChainSigningKeyParam(pub k256::SecretKey);

impl FromStr for CrossChainSigningKeyParam {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(k256::SecretKey::from_slice(&hex::decode(s)?)?))
	}
}

impl CrossChainSigningKeyParam {
	/// Derive corresponding cross-chain public key.
	///
	/// Generates the ECDSA public key from the secret key using SEC1 encoding.
	/// Used for cross-chain identity verification and signature validation.
	///
	/// ## Returns
	///
	/// `CrossChainPublicKey` containing SEC1-encoded public key bytes.
	pub fn vkey(&self) -> CrossChainPublicKey {
		CrossChainPublicKey(self.0.public_key().to_sec1_bytes().to_vec())
	}
}
