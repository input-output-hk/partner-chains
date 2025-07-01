use ogmios_client::query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId};
use ogmios_client::query_network::QueryNetwork;
use ogmios_client::transactions::Transactions;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::cardano_keys::CardanoPaymentSigningKey;
use partner_chains_cardano_offchain::csl::NetworkTypeExt;
use partner_chains_cardano_offchain::multisig::MultiSigSmartContractResult;
use partner_chains_cardano_offchain::permissioned_candidates::{
	get_permissioned_candidates, upsert_permissioned_candidates,
};
use serde::{Deserialize, Serialize};
use sidechain_domain::{PermissionedCandidateData, UtxoId};
use sp_core::crypto::AccountId32;
use sp_core::{ecdsa, ed25519, sr25519};
use sp_runtime::traits::{IdentifyAccount, OpaqueKeys};
use std::fmt::{Display, Formatter};

use crate::cmd_traits::{GetPermissionedCandidates, UpsertPermissionedCandidates};

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

impl From<&sidechain_domain::PermissionedCandidateData> for PermissionedCandidateKeys {
	fn from(value: &sidechain_domain::PermissionedCandidateData) -> Self {
		Self {
			sidechain_pub_key: sp_core::bytes::to_hex(&value.sidechain_public_key.0, false),
			aura_pub_key: sp_core::bytes::to_hex(&value.aura_public_key.0, false),
			grandpa_pub_key: sp_core::bytes::to_hex(&value.grandpa_public_key.0, false),
		}
	}
}

#[derive(Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub(crate) enum ParsedPermissionedCandidatesKeys {
	V0(ParsedPermissionedCandidatesKeysV0),
	V1(ParsedPermissionedCandidatesKeysV1),
}

#[derive(Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub(crate) struct ParsedPermissionedCandidatesKeysV0 {
	pub sidechain: ecdsa::Public,
	pub aura: sr25519::Public,
	pub grandpa: ed25519::Public,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub(crate) struct ParsedPermissionedCandidatesKeysV1 {
	pub keys: Vec<([u8; 4], Vec<u8>)>,
}

impl ParsedPermissionedCandidatesKeys {
	pub fn session_keys<SessionKeys: OpaqueKeys>(&self) -> SessionKeys {
		unimplemented!()
		// SessionKeys::from((sr25519::Public::from(self.aura), ed25519::Public::from(self.grandpa)))
	}

	pub fn account_id_32(&self) -> AccountId32 {
		sp_runtime::MultiSigner::from(self.sidechain).into_account()
	}
}

impl From<ParsedPermissionedCandidatesKeysV0> for ParsedPermissionedCandidatesKeys {
	fn from(value: ParsedPermissionedCandidatesKeysV0) -> Self {
		Self::V0(value)
	}
}

impl From<ParsedPermissionedCandidatesKeysV1> for ParsedPermissionedCandidatesKeys {
	fn from(value: ParsedPermissionedCandidatesKeysV1) -> Self {
		Self::V1(value)
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
		Ok(ParsedPermissionedCandidatesKeysV0 { sidechain, aura, grandpa }.into())
	}
}

impl From<&ParsedPermissionedCandidatesKeys> for sidechain_domain::PermissionedCandidateData {
	fn from(value: &ParsedPermissionedCandidatesKeys) -> Self {
		value.clone().into()
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

impl<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId>
	UpsertPermissionedCandidates for C
{
	async fn upsert_permissioned_candidates(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		candidates: &[PermissionedCandidateData],
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
		upsert_permissioned_candidates(
			genesis_utxo,
			candidates,
			payment_signing_key,
			self,
			&await_tx,
		)
		.await
	}
}

impl<C: QueryLedgerState + QueryNetwork> GetPermissionedCandidates for C {
	async fn get_permissioned_candidates(
		&self,
		genesis_utxo: UtxoId,
	) -> anyhow::Result<Option<Vec<PermissionedCandidateData>>> {
		let network = self.shelley_genesis_configuration().await?.network.to_csl();
		get_permissioned_candidates(genesis_utxo, network, self).await
	}
}
