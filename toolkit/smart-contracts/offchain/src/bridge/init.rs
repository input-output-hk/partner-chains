use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	multisig::MultiSigSmartContractResult,
	plutus_script,
	versioning_system::{ScriptData, initialize_script},
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use raw_scripts::{
	ILLIQUID_CIRCULATION_SUPPLY_AUTHORITY_TOKEN_POLICY, ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
	ScriptId,
};
use sidechain_domain::UtxoId;

/// Stores smart contracts used for bridge (Illiquid Circulation Supply) in the versioning system.
/// Scripts stored are:
///  - Illiquid Circulation Supply Validator
///  - Illiquid Circulation Auth Token Policy
pub async fn init_ics_scripts<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Vec<MultiSigSmartContractResult>> {
	let ics_validator = ScriptData::new(
		"Illiquid Circulation Supply Validator",
		ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR.0.to_vec(),
		ScriptId::IlliquidCirculationSupplyValidator,
	);
	let ics_auth_token_policy = ScriptData::new(
		"Illiquid Circulation Supply Auth Token Policy",
		plutus_script![
			ILLIQUID_CIRCULATION_SUPPLY_AUTHORITY_TOKEN_POLICY,
			ScriptId::IlliquidCirculationSupplyAuthorityTokenPolicy
		]?
		.bytes
		.to_vec(),
		ScriptId::IlliquidCirculationSupplyAuthorityTokenPolicy,
	);

	Ok(vec![
		initialize_script(ics_validator, genesis_utxo, payment_key, client, await_tx).await?,
		initialize_script(ics_auth_token_policy, genesis_utxo, payment_key, client, await_tx)
			.await?,
	]
	.into_iter()
	.flatten()
	.collect())
}
