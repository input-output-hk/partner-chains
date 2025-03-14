use secp256k1::{PublicKey, SecretKey};
use sidechain_domain::*;
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
pub struct StakeSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakeSigningKeyParam {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		Ok(Self(ed25519_zebra::SigningKey::try_from(hex::decode(trimmed)?.as_slice())?))
	}
}

impl StakeSigningKeyParam {
	pub fn vkey(&self) -> StakePublicKey {
		StakePublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}

#[derive(Clone, Debug)]
pub struct StakePoolSigningKeyParam(pub ed25519_zebra::SigningKey);

impl FromStr for StakePoolSigningKeyParam {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let trimmed = s.trim_start_matches("0x");
		Ok(Self(ed25519_zebra::SigningKey::try_from(hex::decode(trimmed)?.as_slice())?))
	}
}

impl StakePoolSigningKeyParam {
	pub fn vkey(&self) -> StakePoolPublicKey {
		StakePoolPublicKey(ed25519_zebra::VerificationKey::from(&self.0).into())
	}
}
