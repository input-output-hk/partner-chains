use crate::csl::{NetworkTypeExt, OgmiosUtxoExt};
use crate::plutus_script;
use crate::scripts_data;
use cardano_serialization_lib::*;
use ogmios_client::{
	query_ledger_state::QueryLedgerState, query_network::QueryNetwork, types::OgmiosUtxo,
};
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug)]
pub(crate) struct GovernanceData {
	pub(crate) policy_script: plutus_script::PlutusScript,
	pub(crate) utxo: OgmiosUtxo,
}

impl GovernanceData {
	pub fn utxo_id(&self) -> sidechain_domain::UtxoId {
		self.utxo.utxo_id()
	}

	pub(crate) fn utxo_id_as_tx_input(&self) -> TransactionInput {
		TransactionInput::new(
			&TransactionHash::from_bytes(self.utxo_id().tx_hash.0.to_vec()).unwrap(),
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
		let policy_script = read_policy(&utxo)?;
		Ok(GovernanceData { policy_script, utxo })
	}
}

fn read_policy(governance_utxo: &OgmiosUtxo) -> Result<plutus_script::PlutusScript, JsError> {
	let script = governance_utxo
		.script
		.clone()
		.ok_or_else(|| JsError::from_str("No 'script' in governance UTXO"))?;
	plutus_script::PlutusScript::from_ogmios(script).map_err(|e| {
		JsError::from_str(&format!(
			"Cannot convert script from UTXO {}: {}",
			governance_utxo.to_domain(),
			e
		))
	})
}
