use plutus_datum_derive::ToDatum;
use secp256k1::{PublicKey, SecretKey};
use sidechain_domain::*;
use sidechain_domain::{SidechainPublicKey, StakePoolPublicKey, StakePublicKey};
use std::{
	convert::Infallible,
	fmt::Display,
	io::{self, ErrorKind},
	str::FromStr,
};

pub mod register1;
pub mod register2;
pub mod register3;

#[derive(Clone, Debug, ToDatum)]
pub struct RegisterValidatorMessage {
	pub genesis_utxo: UtxoId,
	pub sidechain_pub_key: SidechainPublicKey,
	pub registration_utxo: UtxoId,
}

#[derive(Clone, Debug)]
pub struct SidechainPublicKeyParam(pub SidechainPublicKey);

impl Display for SidechainPublicKeyParam {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "0x{}", hex::encode(&self.0 .0))
	}
}

impl FromStr for SidechainPublicKeyParam {
	type Err = secp256k1::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		let pk = PublicKey::from_str(trimmed)?;
		Ok(SidechainPublicKeyParam(SidechainPublicKey(pk.serialize().to_vec())))
	}
}

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

#[derive(Debug, thiserror::Error)]
pub enum Ed25519SigningKeyError {
	#[error("{0}")]
	HexError(#[from] hex::FromHexError),
	#[error("{0}")]
	Ed25519Error(#[from] ed25519_zebra::Error),
}

impl From<Ed25519SigningKeyError> for io::Error {
	fn from(value: Ed25519SigningKeyError) -> Self {
		io::Error::new(ErrorKind::InvalidInput, value)
	}
}

#[derive(Clone, Debug)]
pub struct StakePoolSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakePoolSigningKeyParam {
	type Err = Ed25519SigningKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		Ok(Self(ed25519_zebra::SigningKey::try_from(hex::decode(trimmed)?.as_slice())?))
	}
}

impl From<[u8; 32]> for StakePoolSigningKeyParam {
	fn from(key: [u8; 32]) -> Self {
		Self(ed25519_zebra::SigningKey::from(key))
	}
}
