use serde::{Deserialize, Serialize};
use sp_core::crypto::AccountId32;
use sp_core::{ecdsa, ed25519, sr25519};
use sp_runtime::traits::IdentifyAccount;
use std::fmt::{Display, Formatter};

#[derive(Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Serialize)]
pub(crate) struct PermissionedCandidateKeys {
	/// 0x prefixed hex representation of the ECDSA public key
	pub sidechain_pub_key: String,
	/// 0x prefixed hex representation of the sr25519 public key
	pub aura_pub_key: String,
	/// 0x prefixed hex representation of the Ed25519 public key
	pub grandpa_pub_key: String,
}

impl Display for PermissionedCandidateKeys {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Partner Chains Key: {}, AURA: {}, GRANDPA: {}",
			self.sidechain_pub_key, self.aura_pub_key, self.grandpa_pub_key
		)
	}
}

impl From<&ParsedPermissionedCandidatesKeys> for PermissionedCandidateKeys {
	fn from(value: &ParsedPermissionedCandidatesKeys) -> Self {
		Self {
			sidechain_pub_key: sp_core::bytes::to_hex(&value.sidechain.0, false),
			aura_pub_key: sp_core::bytes::to_hex(&value.aura.0, false),
			grandpa_pub_key: sp_core::bytes::to_hex(&value.grandpa.0, false),
		}
	}
}

#[derive(Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub(crate) struct ParsedPermissionedCandidatesKeys {
	pub sidechain: ecdsa::Public,
	pub aura: sr25519::Public,
	pub grandpa: ed25519::Public,
}

impl ParsedPermissionedCandidatesKeys {
	pub fn session_keys<SessionKeys: From<(sr25519::Public, ed25519::Public)>>(
		&self,
	) -> SessionKeys {
		SessionKeys::from((sr25519::Public::from(self.aura), ed25519::Public::from(self.grandpa)))
	}

	pub fn account_id_32(&self) -> AccountId32 {
		sp_runtime::MultiSigner::from(self.sidechain).into_account()
	}
}

impl TryFrom<&PermissionedCandidateKeys> for ParsedPermissionedCandidatesKeys {
	type Error = anyhow::Error;

	fn try_from(value: &PermissionedCandidateKeys) -> Result<Self, Self::Error> {
		let sidechain = parse_ecdsa(&value.sidechain_pub_key).ok_or(anyhow::Error::msg(
			format!("{} is invalid ECDSA public key", value.sidechain_pub_key),
		))?;
		let aura = parse_sr25519(&value.aura_pub_key).ok_or(anyhow::Error::msg(format!(
			"{} is invalid sr25519 public key",
			value.aura_pub_key
		)))?;
		let grandpa = parse_ed25519(&value.grandpa_pub_key).ok_or(anyhow::Error::msg(format!(
			"{} is invalid Ed25519 public key",
			value.grandpa_pub_key
		)))?;
		Ok(Self { sidechain, aura, grandpa })
	}
}

fn parse_ecdsa(value: &str) -> Option<ecdsa::Public> {
	let bytes = sp_core::bytes::from_hex(value).ok()?;
	Some(ecdsa::Public::from(<[u8; 33]>::try_from(bytes).ok()?))
}

fn parse_sr25519(value: &str) -> Option<sr25519::Public> {
	let bytes = sp_core::bytes::from_hex(value).ok()?;
	Some(sr25519::Public::from(<[u8; 32]>::try_from(bytes).ok()?))
}

fn parse_ed25519(value: &str) -> Option<ed25519::Public> {
	let bytes = sp_core::bytes::from_hex(value).ok()?;
	Some(ed25519::Public::from(<[u8; 32]>::try_from(bytes).ok()?))
}
