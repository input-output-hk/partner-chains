use crate::csl::{NetworkTypeExt, OgmiosUtxoExt};
use crate::plutus_script;
use crate::scripts_data;
use cardano_serialization_lib::*;
use ogmios_client::types::NativeScript as OgmiosNativeScript;
use ogmios_client::{
	query_ledger_state::QueryLedgerState, query_network::QueryNetwork, types::OgmiosUtxo,
};
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use partner_chains_plutus_data::PlutusDataExtensions as _;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug)]
pub(crate) struct GovernanceData {
	pub(crate) policy: GovernancePolicyScript,
	pub(crate) utxo: OgmiosUtxo,
}

/// The supported Governance Policies are:
/// - Plutus MultiSig implementation from partner-chain-smart-contracts
/// - Native Script `atLeast` with only simple `sig` type of inner `scripts` field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum GovernancePolicyScript {
	MultiSig(PartnerChainsMultisigPolicy),
	AtLeastNNativeScript(SimpleAtLeastN),
}

impl GovernancePolicyScript {
	pub(crate) fn script(&self) -> crate::csl::Script {
		match self {
			Self::MultiSig(policy) => crate::csl::Script::Plutus(policy.script.clone()),
			Self::AtLeastNNativeScript(policy) => {
				crate::csl::Script::Native(policy.to_csl_native_script())
			},
		}
	}
}

/// Plutus MultiSig implemented in partner-chains-smart-contracts repo,
/// it is legacy and ideally should have been used only with a single key in the `governance init`.
/// It allows to mint the governance token only if the transaction in `required_singers` field
/// has at least `threshold` key hashes that are in the `key_hashes` list.
/// `threshold` and `key_hashes` are applied Plutus Data applied to the script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PartnerChainsMultisigPolicy {
	pub(crate) script: plutus_script::PlutusScript,
	pub(crate) key_hashes: Vec<[u8; 28]>,
	pub(crate) threshold: u32,
}

/// This represent Cardano Native Script of type `atLeast`, where each of `scripts` has to be
/// of type `sig`. We call them `key_hashes` to match our Partner Chains Plutus MultiSig policy.
/// `threshold` field of this struct is mapped to `required` field in the simple script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SimpleAtLeastN {
	pub(crate) threshold: u32,
	pub(crate) key_hashes: Vec<[u8; 28]>,
}

impl SimpleAtLeastN {
	pub fn to_csl_native_script(&self) -> NativeScript {
		let mut native_scripts = NativeScripts::new();
		for key_hash in self.key_hashes.clone() {
			native_scripts.add(&NativeScript::new_script_pubkey(&ScriptPubkey::new(
				&Ed25519KeyHash::from(key_hash),
			)))
		}
		NativeScript::new_script_n_of_k(&ScriptNOfK::new(self.threshold, &native_scripts))
	}
}

impl GovernanceData {
	pub fn utxo_id(&self) -> sidechain_domain::UtxoId {
		self.utxo.utxo_id()
	}

	pub(crate) fn utxo_id_as_tx_input(&self) -> TransactionInput {
		TransactionInput::new(
			&TransactionHash::from(self.utxo_id().tx_hash.0),
			self.utxo_id().index.0.into(),
		)
	}

	async fn get_governance_utxo<T: QueryLedgerState + QueryNetwork>(
		genesis_utxo: UtxoId,
		client: &T,
	) -> Result<OgmiosUtxo, JsError> {
		let network = client
			.shelley_genesis_configuration()
			.await
			.map_err(|e| {
				JsError::from_str(&format!("Could not get Shelley Genesis Configuration: {}", e))
			})?
			.network;

		let (_, version_oracle_policy, validator_address) =
			scripts_data::version_scripts_and_address(genesis_utxo, network.to_csl()).map_err(
				|e| {
					JsError::from_str(&format!(
						"Could not get Version Oracle Script Data for: {}, {}",
						genesis_utxo, e
					))
				},
			)?;

		let utxos = client.query_utxos(&[validator_address.clone()]).await.map_err(|e| {
			JsError::from_str(&format!(
				"Could not query UTXOs Governance Validator at {}: {}",
				validator_address, e
			))
		})?;

		utxos
		.into_iter()
		.find(|utxo| {
			let correct_datum =
				utxo.get_plutus_data()
					.and_then(|plutus_data| VersionOracleDatum::try_from(plutus_data).ok())
					.map(|data| data.version_oracle == 32)
					.unwrap_or(false);

			let contains_version_oracle_token =
				utxo.value.native_tokens.contains_key(&version_oracle_policy.script_hash());
			correct_datum && contains_version_oracle_token
		})
		.ok_or_else(|| JsError::from_str("Could not find governance versioning UTXO. This most likely means that governance was not properly set up on Cardano using governance init command."))
	}

	pub(crate) async fn get<T: QueryLedgerState + QueryNetwork>(
		genesis_utxo: UtxoId,
		client: &T,
	) -> Result<GovernanceData, JsError> {
		let utxo = Self::get_governance_utxo(genesis_utxo, client).await?;
		let policy = read_policy(&utxo)?;
		Ok(GovernanceData { policy, utxo })
	}
}

fn read_policy(governance_utxo: &OgmiosUtxo) -> Result<GovernancePolicyScript, JsError> {
	let script = governance_utxo
		.script
		.clone()
		.ok_or_else(|| JsError::from_str("No 'script' in governance UTXO"))?;
	let plutus_multisig = plutus_script::PlutusScript::from_ogmios(script.clone())
		.ok()
		.and_then(parse_pc_multisig);
	let policy_script = plutus_multisig.or_else(|| parse_simple_at_least_n_native_script(script));
	policy_script.ok_or_else(|| {
		JsError::from_str(&format!(
			"Cannot convert script from UTXO {} into a multisig policy",
			governance_utxo.utxo_id(),
		))
	})
}

/// Returns decoded Governance Authorities and threshold if the policy script is an applied MultiSig policy.
/// Returns None in case decoding failed, perhaps when some other policy is used.
/// This method does not check for the policy itself. If invoked for a different policy, most probably will return None, with some chance of returning trash data.
fn parse_pc_multisig(script: plutus_script::PlutusScript) -> Option<GovernancePolicyScript> {
	script.unapply_data_csl().ok().and_then(|data| data.as_list()).and_then(|list| {
		let mut it = list.into_iter();
		let key_hashes = it.next().and_then(|data| {
			data.as_list().map(|list| {
				list.into_iter()
					.filter_map(|item| item.as_bytes().and_then(|bytes| bytes.try_into().ok()))
					.collect::<Vec<_>>()
			})
		})?;
		let threshold: u32 = it.next().and_then(|t| t.as_u32())?;
		Some(GovernancePolicyScript::MultiSig(PartnerChainsMultisigPolicy {
			script,
			key_hashes,
			threshold,
		}))
	})
}

fn parse_simple_at_least_n_native_script(
	script: ogmios_client::types::OgmiosScript,
) -> Option<GovernancePolicyScript> {
	match script.json {
		Some(OgmiosNativeScript::Some { from, at_least }) => {
			let mut keys = Vec::with_capacity(from.len());
			for x in from {
				let key = match x {
					OgmiosNativeScript::Signature { from: key_hash } => Some(key_hash),
					_ => None,
				}?;
				keys.push(key);
			}
			Some(GovernancePolicyScript::AtLeastNNativeScript(SimpleAtLeastN {
				threshold: at_least,
				key_hashes: keys,
			}))
		},
		_ => None,
	}
}
