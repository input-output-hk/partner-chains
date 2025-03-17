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

/// Plutus MultiSig smart contract implemented in partner-chains-smart-contracts repo
/// is legacy and ideally should have been used only with a single key in `governance init`.
/// It allows minting the governance token only if the transaction in `required_singers` field
/// has at least `threshold` number of key hashes from the `key_hashes` list.
/// `threshold` and `key_hashes` are Plutus Data applied to the script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PartnerChainsMultisigPolicy {
	pub(crate) script: plutus_script::PlutusScript,
	pub(crate) key_hashes: Vec<[u8; 28]>,
	pub(crate) threshold: u32,
}

/// This represents Cardano Native Script of type `atLeast`, where each of `scripts` has to be
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

#[cfg(test)]
mod tests {
	use super::read_policy;
	use crate::{
		governance::{GovernancePolicyScript, PartnerChainsMultisigPolicy, SimpleAtLeastN},
		plutus_script::PlutusScript,
	};
	use hex_literal::hex;
	use ogmios_client::types::OgmiosUtxo;

	#[test]
	fn read_pc_multisig_from_ogmios_utxo() {
		let utxo_json = serde_json::json!({
			"transaction": { "id": "57342ce4f30afa749bd78f0c093609366d997a1c4747d206ec7fd0aea9a35b55" },
			"index": 0,
			"address": "addr_test1wplvesjjxtg8lhyy34ak2dr9l3kz8ged3hajvcvpanfx7rcwzvtc5",
			"value": { "ada": { "lovelace": 1430920 } },
			"script": {
			  "language": "plutus:v2",
				  "cbor": "59020f0100003323322323232323322323232222323232532323355333573466e20cc8c8c88c008004c058894cd4004400c884cc018008c010004c04488004c04088008c01000400840304034403c4c02d24010350543500300d37586ae84008dd69aba1357440026eb0014c040894cd400440448c884c8cd40514cd4c00cc04cc030dd6198009a9803998009a980380411000a40004400290080a400429000180300119112999ab9a33710002900009807a490350543600133003001002301522253350011300f49103505437002215333573466e1d20000041002133005337020089001000980991299a8008806910a999ab9a3371e00a6eb800840404c0100048c8cc8848cc00400c008d55ce80098031aab9e00137540026016446666aae7c00480348cd4030d5d080118019aba2002498c02888cccd55cf8009006119a8059aba100230033574400493119319ab9c00100512200212200130062233335573e0024010466a00e6eb8d5d080118019aba20020031200123300122337000040029000180191299a800880211099a802801180200089100109109119800802001919180080091198019801001000a615f9f9f581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b581c01010101010101010101010101010101010101010101010101010101581c02020202020202020202020202020202020202020202020202020202ff02ff0001"
			}
		});
		let ogmios_utxo: OgmiosUtxo = serde_json::from_value(utxo_json).unwrap();
		let policy = read_policy(&ogmios_utxo).unwrap();
		assert_eq!(
			policy,
			GovernancePolicyScript::MultiSig(PartnerChainsMultisigPolicy {
				script: PlutusScript {
					bytes: hex!("59020f0100003323322323232323322323232222323232532323355333573466e20cc8c8c88c008004c058894cd4004400c884cc018008c010004c04488004c04088008c01000400840304034403c4c02d24010350543500300d37586ae84008dd69aba1357440026eb0014c040894cd400440448c884c8cd40514cd4c00cc04cc030dd6198009a9803998009a980380411000a40004400290080a400429000180300119112999ab9a33710002900009807a490350543600133003001002301522253350011300f49103505437002215333573466e1d20000041002133005337020089001000980991299a8008806910a999ab9a3371e00a6eb800840404c0100048c8cc8848cc00400c008d55ce80098031aab9e00137540026016446666aae7c00480348cd4030d5d080118019aba2002498c02888cccd55cf8009006119a8059aba100230033574400493119319ab9c00100512200212200130062233335573e0024010466a00e6eb8d5d080118019aba20020031200123300122337000040029000180191299a800880211099a802801180200089100109109119800802001919180080091198019801001000a615f9f9f581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b581c01010101010101010101010101010101010101010101010101010101581c02020202020202020202020202020202020202020202020202020202ff02ff0001").to_vec(),
				 language: cardano_serialization_lib::Language::new_plutus_v2()
				},
				key_hashes: vec![hex!("e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"), [1u8; 28], [2u8; 28]],
				threshold: 2
			})
		)
	}

	#[test]
	fn read_simple_at_least_n_native_script_from_ogmios_utxo() {
		let utxo_json = serde_json::json!({
			"transaction": { "id": "57342ce4f30afa749bd78f0c093609366d997a1c4747d206ec7fd0aea9a35b55" },
			"index": 0,
			"address": "addr_test1wplvesjjxtg8lhyy34ak2dr9l3kz8ged3hajvcvpanfx7rcwzvtc5",
			"value": { "ada": { "lovelace": 1430920 } },
			"script": {
				"language": "native",
				"json": {
					"clause": "some",
					"atLeast": 2,
					"from": [
						{
							"clause": "signature",
							"from": "e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
						},
						{
							"clause": "signature",
							"from": "a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7"
						},
						{
							"clause": "signature",
							"from": "b1b2b3b4b5b6b7b1b2b3b4b5b6b7b1b2b3b4b5b6b7b1b2b3b4b5b6b7"
						}
					  ]
				},
				"cbor": "830301818200581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
			}
		});
		let ogmios_utxo: OgmiosUtxo = serde_json::from_value(utxo_json).unwrap();
		let policy = read_policy(&ogmios_utxo).unwrap();
		assert_eq!(
			policy,
			GovernancePolicyScript::AtLeastNNativeScript(SimpleAtLeastN {
				threshold: 2,
				key_hashes: vec![
					hex!("e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"),
					hex!("a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7a1a2a3a4a5a6a7"),
					hex!("b1b2b3b4b5b6b7b1b2b3b4b5b6b7b1b2b3b4b5b6b7b1b2b3b4b5b6b7")
				]
			})
		)
	}
}
