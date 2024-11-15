use secp256k1::{PublicKey, SecretKey};
use sidechain_domain::SidechainPublicKey;
use std::convert::Infallible;
use std::fmt::Display;
use std::io;
use std::io::ErrorKind;
use std::str::FromStr;

#[derive(Clone, Debug)]
pub struct SidechainSigningKeyParam(pub SecretKey);

impl SidechainSigningKeyParam {
	pub fn to_pub_key(&self) -> PublicKey {
		PublicKey::from_secret_key_global(&self.0)
	}
}

impl FromStr for SidechainSigningKeyParam {
	type Err = secp256k1::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		let pair = SecretKey::from_str(trimmed)?;
		Ok(SidechainSigningKeyParam(pair))
	}
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
pub enum MainchainKeyError {
	#[error("{0}")]
	HexError(#[from] hex::FromHexError),
	#[error("{0}")]
	Ed25519Error(#[from] ed25519_zebra::Error),
}

impl From<MainchainKeyError> for io::Error {
	fn from(value: MainchainKeyError) -> Self {
		io::Error::new(ErrorKind::InvalidInput, value)
	}
}

#[derive(Clone, Debug)]
pub struct MainchainSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for MainchainSigningKeyParam {
	type Err = MainchainKeyError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		let key = ed25519_zebra::SigningKey::try_from(hex::decode(trimmed)?.as_slice())?;
		Ok(MainchainSigningKeyParam(key))
	}
}

impl From<[u8; 32]> for MainchainSigningKeyParam {
	fn from(key: [u8; 32]) -> Self {
		MainchainSigningKeyParam(ed25519_zebra::SigningKey::from(key))
	}
}
