//! CLI parameter wrappers for cryptographic keys with secure parsing and validation.

use sidechain_domain::*;
use std::convert::Infallible;
use std::fmt::Display;
use std::io;
use std::io::ErrorKind;
use std::str::FromStr;

/// ECDSA private key wrapper for Partner Chain validator operations.
#[derive(Clone, Debug)]
pub struct SidechainSigningKeyParam(pub secp256k1::SecretKey);

impl SidechainSigningKeyParam {
	/// Derives the corresponding ECDSA public key.
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

/// ECDSA public key wrapper for Partner Chain operations.
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

/// Generic string wrapper for public keys without cryptographic validation.
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

/// Parses hex string into Ed25519 signing key with validation.
pub(crate) fn parse_zebra_signing_key(
	s: &str,
) -> Result<ed25519_zebra::SigningKey, Ed25519SigningKeyError> {
	let trimmed = s.trim_start_matches("0x");
	Ok(ed25519_zebra::SigningKey::try_from(hex::decode(trimmed)?.as_slice())?)
}

/// Ed25519 private key wrapper for Cardano stake pool operations.
#[derive(Clone, Debug)]
pub struct StakePoolSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakePoolSigningKeyParam {
	type Err = Ed25519SigningKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(parse_zebra_signing_key(s)?))
	}
}

impl From<[u8; 32]> for StakePoolSigningKeyParam {
	/// Creates signing key from 32-byte array.
	fn from(key: [u8; 32]) -> Self {
		Self(ed25519_zebra::SigningKey::from(key))
	}
}

impl StakePoolSigningKeyParam {
	/// Derives the corresponding public key.
	pub fn vkey(&self) -> StakePoolPublicKey {
		StakePoolPublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}

/// Ed25519 private key wrapper for Cardano staking operations.
#[derive(Clone, Debug)]
pub struct StakeSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakeSigningKeyParam {
	type Err = Ed25519SigningKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(parse_zebra_signing_key(s)?))
	}
}

impl StakeSigningKeyParam {
	/// Derives the corresponding public key.
	pub fn vkey(&self) -> StakePublicKey {
		StakePublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}

/// ECDSA private key wrapper for cross-chain operations.
#[derive(Clone, Debug)]
pub struct CrossChainSigningKeyParam(pub k256::SecretKey);

impl FromStr for CrossChainSigningKeyParam {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self(k256::SecretKey::from_slice(&hex::decode(s)?)?))
	}
}

impl CrossChainSigningKeyParam {
	/// Derives the corresponding ECDSA public key.
	pub fn vkey(&self) -> CrossChainPublicKey {
		CrossChainPublicKey(self.0.public_key().to_sec1_bytes().to_vec())
	}
}
