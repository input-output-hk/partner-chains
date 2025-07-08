use crate::cmd_traits::{GetPermissionedCandidates, UpsertPermissionedCandidates};
use ogmios_client::query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId};
use ogmios_client::query_network::QueryNetwork;
use ogmios_client::transactions::Transactions;
use parity_scale_codec::Decode;
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
use std::marker::PhantomData;

/// Struct that holds permissioned candidates keys in raw string format
#[derive(Debug, Deserialize, Eq, PartialEq, PartialOrd, Ord, Serialize)]
pub struct PermissionedCandidateKeys {
	/// All keys associated with given candidate
	pub keys: Vec<([u8; 4], Vec<u8>)>,
}

impl Display for PermissionedCandidateKeys {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Keys with key type: {:?}", self.keys)
	}
}

impl From<&sidechain_domain::PermissionedCandidateData> for PermissionedCandidateKeys {
	fn from(value: &sidechain_domain::PermissionedCandidateData) -> Self {
		match value {
			PermissionedCandidateData::V0(permissioned_candidate_data_v0) => {
				PermissionedCandidateKeys {
					keys: vec![
						(*b"crch", permissioned_candidate_data_v0.sidechain_public_key.0.clone()),
						(*b"aura", permissioned_candidate_data_v0.aura_public_key.0.clone()),
						(*b"gran", permissioned_candidate_data_v0.grandpa_public_key.0.clone()),
					],
				}
			},
			PermissionedCandidateData::V1(permissioned_candidate_data_v1) => {
				PermissionedCandidateKeys { keys: permissioned_candidate_data_v1.keys.clone() }
			},
		}
	}
}

/// Groups together keys of permissioned candidates.
#[derive(Debug, Deserialize, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct ParsedPermissionedCandidatesKeys<AuthorityKeys> {
	sidechain_key: ecdsa::Public,
	keys: Vec<u8>,
	_phantom: PhantomData<AuthorityKeys>,
}

impl<AuthorityKeys> ParsedPermissionedCandidatesKeys<AuthorityKeys> {
	/// Permissioned candidate set of session keys
	pub fn session_keys(&self) -> AuthorityKeys {
		// let encoded =
		// 	self.session_keysss().iter().fold(Vec::new(), |mut encoded, (_key_type, key)| {
		// 		encoded.extend(key);
		// 		encoded
		// 	});

		// let sessions_keys = SessionKeys::decode(&mut &encoded[..]).ok()?;

		unimplemented!();

		// Some(sessions_keys)
	}

	/// Permissioned candidate set of session keys
	pub fn session_keysss(&self) -> Vec<([u8; 4], Vec<u8>)> {
		// self.keys.clone()
		unimplemented!()
	}

	/// Permissioned Candidate partner-chain (sidechain) key
	pub fn sidechain_key(&self) -> ecdsa::Public {
		self.sidechain_key
	}

	/// Permissioned Candidate partner-chain (sidechain) key mapped to AccountId32
	pub fn account_id_32(&self) -> AccountId32 {
		sp_runtime::MultiSigner::from(self.sidechain_key()).into_account()
	}
}

impl<AuthorityKeys> TryFrom<&PermissionedCandidateKeys>
	for ParsedPermissionedCandidatesKeys<AuthorityKeys>
{
	type Error = anyhow::Error;

	fn try_from(value: &PermissionedCandidateKeys) -> Result<Self, Self::Error> {
		let (_sidechain_key_type, sidechain_key) = value
			.keys
			.iter()
			.find(|key| key.0 == *b"crch")
			.ok_or(anyhow::Error::msg(format!("Missing ECDSA sidechain key")))
			.cloned()?;

		let sidechain_key = <[u8; 33]>::try_from(sidechain_key).map_err(|sidechain_key| {
			anyhow::Error::msg(format!("{:?} is invalid ECDSA public key", sidechain_key))
		})?;

		// let sidechain_key = ecdsa::Public::from(sidechain_key).into();

		unimplemented!();

		// TODO: should we filter out crch key?
		// let keys = value
		// 	.keys
		// 	.iter()
		// 	.filter(|(key_type, _key)| key_type != b"crch")
		// 	.cloned()
		// 	.collect();

		// Ok(Self { sidechain_key, keys })
	}
}

impl<AuthorityKeys> From<&ParsedPermissionedCandidatesKeys<AuthorityKeys>>
	for sidechain_domain::PermissionedCandidateData
{
	fn from(value: &ParsedPermissionedCandidatesKeys<AuthorityKeys>) -> Self {
		value.into()
	}
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
