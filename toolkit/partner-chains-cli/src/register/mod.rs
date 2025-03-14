use plutus_datum_derive::ToDatum;
use secp256k1::{PublicKey, SecretKey};
use sidechain_domain::*;
use sidechain_domain::{SidechainPublicKey, StakePoolPublicKey, StakePublicKey};
use std::{convert::Infallible, fmt::Display, str::FromStr};

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

#[derive(Clone, Debug)]
pub struct StakePoolSigningKeyParam(pub ed25519_zebra::SigningKey);

impl From<[u8; 32]> for StakePoolSigningKeyParam {
	fn from(key: [u8; 32]) -> Self {
		Self(ed25519_zebra::SigningKey::from(key))
	}
}
