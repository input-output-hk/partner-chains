use crate::cmd_traits::{GetPermissionedCandidates, UpsertPermissionedCandidates};
use authority_selection_inherents::MaybeFromCandidateKeys;
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
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::{CandidateKey, CandidateKeys, PermissionedCandidateData, UtxoId};
use sp_core::crypto::AccountId32;
use sp_core::ecdsa;
use sp_runtime::traits::{IdentifyAccount, OpaqueKeys};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// Struct that holds permissioned candidates keys in raw string-ish formats
#[derive(Debug, Serialize, Deserialize)]
pub struct PermissionedCandidateKeys {
	/// 0x prefixed hex representation of the ECDSA public key
	pub partner_chains_key: ByteString,
	/// Keys are text representation of key type id, values are key bytes
	pub keys: BTreeMap<String, ByteString>,
}

impl PermissionedCandidateKeys {
	fn keys_sorted(&self) -> Vec<(String, ByteString)> {
		let mut v: Vec<_> = self.keys.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
		v.sort();
		v
	}
}

impl Display for PermissionedCandidateKeys {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Partner Chains Key: {}", self.partner_chains_key.to_hex_string())?;
		for (id, bytes) in self.keys_sorted().iter() {
			write!(f, ", {id}: {}", bytes.to_hex_string())?
		}
		Ok(())
	}
}

impl From<&sidechain_domain::PermissionedCandidateData> for PermissionedCandidateKeys {
	fn from(value: &sidechain_domain::PermissionedCandidateData) -> Self {
		Self {
			partner_chains_key: ByteString::from(value.sidechain_public_key.0.clone()),
			keys: value
				.keys
				.0
				.iter()
				.map(|ck| {
					(
						String::from_utf8(ck.id.to_vec()).expect("key type ids are valid utf-8"),
						ByteString::from(ck.bytes.clone()),
					)
				})
				.collect(),
		}
	}
}

impl TryFrom<&PermissionedCandidateKeys> for sidechain_domain::CandidateKeys {
	type Error = String;

	fn try_from(value: &PermissionedCandidateKeys) -> Result<Self, Self::Error> {
		let mut acc = vec![];
		for (k, v) in value.keys.iter() {
			let id: [u8; 4] = k
				.as_bytes()
				.try_into()
				.map_err(|_| format!("Could not parse key type id: '{k}'"))?;
			let bytes = v.0.clone();
			acc.push(CandidateKey { id, bytes });
		}
		Ok(CandidateKeys(acc))
	}
}

/// Groups together keys of permissioned candidates.
#[derive(Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ParsedPermissionedCandidatesKeys<Keys> {
	/// Polkadot identity of the permissioned candidate (aka. partner-chain key)
	pub sidechain: ecdsa::Public,
	/// Keys of the candidate, other than the partner chain key
	pub keys: Keys,
}

impl<T> ParsedPermissionedCandidatesKeys<T> {
	/// Permissioned Candidate partner-chain (sidechain) key mapped to AccountId32
	pub fn account_id_32(&self) -> AccountId32 {
		sp_runtime::MultiSigner::from(self.sidechain).into_account()
	}
}

impl<Keys: MaybeFromCandidateKeys> TryFrom<&PermissionedCandidateKeys>
	for ParsedPermissionedCandidatesKeys<Keys>
{
	type Error = anyhow::Error;

	fn try_from(value: &PermissionedCandidateKeys) -> Result<Self, Self::Error> {
		let sidechain = parse_ecdsa(&value.partner_chains_key.0).ok_or(anyhow::Error::msg(
			format!("'{}' is invalid ECDSA public key", value.partner_chains_key.to_hex_string()),
		))?;
		let candidate_keys = CandidateKeys::try_from(value).map_err(|e| anyhow::anyhow!(e))?;
		let keys = MaybeFromCandidateKeys::maybe_from(&candidate_keys)
			.ok_or(anyhow::anyhow!("Could not parse candidate keys!"))?;
		Ok(Self { sidechain, keys })
	}
}

impl<Keys: OpaqueKeys> From<&ParsedPermissionedCandidatesKeys<Keys>>
	for sidechain_domain::PermissionedCandidateData
{
	fn from(value: &ParsedPermissionedCandidatesKeys<Keys>) -> Self {
		let keys = Keys::key_ids()
			.iter()
			.map(|key_id| {
				let bytes = value.keys.get_raw(*key_id).to_vec();
				CandidateKey { id: key_id.0, bytes }
			})
			.collect();
		Self {
			sidechain_public_key: sidechain_domain::SidechainPublicKey(value.sidechain.0.to_vec()),
			keys: CandidateKeys(keys),
		}
	}
}

fn parse_ecdsa(bytes: &[u8]) -> Option<ecdsa::Public> {
	Some(ecdsa::Public::from(<[u8; 33]>::try_from(bytes).ok()?))
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
