//! Building and submitting of the transaction that changes the current governance authority
//!
//! The transaction:
//! 1. Burns the previous governance UTXO from the version oracle validator address
//! 2. Mints exactly 1 multi-sig policy token as authentication
//! 3. Produces a new governance UTXO at the version oracle validator address with a version oracle
//!    Plutus datum attached that contains the script ID (32) and policy hash.
use crate::{
	await_tx::AwaitTx,
	csl::{
		get_validator_budgets, InputsBuilderExt, ScriptExUnits, TransactionBuilderExt,
		TransactionContext,
	},
	init_governance::{self, transaction::version_oracle_datum_output, GovernanceData},
	plutus_script::PlutusScript,
	scripts_data::multisig_governance_policy_configuration,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	ExUnits, Language, PlutusData, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosTx,
};
use sidechain_domain::{MainchainAddressHash, MainchainPrivateKey, McTxHash, UtxoId, UtxoIndex};

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_values;

pub async fn run_update_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	new_governance_authority: MainchainAddressHash,
	payment_key: MainchainPrivateKey,
	genesis_utxo_id: UtxoId,
	client: &T,
	await_tx: A,
) -> anyhow::Result<OgmiosTx> {
	let tx_context = TransactionContext::for_payment_key(payment_key.0, client).await?;

	let governance_data = init_governance::get_governance_data(genesis_utxo_id, client).await?;

	let tx = update_governance_tx(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		genesis_utxo_id,
		new_governance_authority,
		&tx_context,
		&governance_data,
		ExUnits::new(&0u64.into(), &0u64.into()),
		ExUnits::new(&0u64.into(), &0u64.into()),
	)?;

	let costs = client.evaluate_transaction(&tx.to_bytes()).await?;

	let ScriptExUnits { mint_ex_units, spend_ex_units } = get_validator_budgets(costs);

	let [mint_cost] = &mint_ex_units[..] else {
		return Err(anyhow!("Error retrieving witness costs: mint cost data missing."));
	};
	let [spend_cost] = &spend_ex_units[..] else {
		return Err(anyhow!("Error retrieving witness costs: spend cost data missing."));
	};

	let tx = update_governance_tx(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		genesis_utxo_id,
		new_governance_authority,
		&tx_context,
		&governance_data,
		mint_cost.clone(),
		spend_cost.clone(),
	)?;
	let signed_tx = tx_context.sign(&tx);

	let response = client.submit_transaction(&signed_tx.to_bytes()).await?;
	log::info!("Submitted transaction: {}", hex::encode(response.transaction.id));

	await_tx
		.await_tx_output(
			client,
			UtxoId { tx_hash: McTxHash(response.transaction.id), index: UtxoIndex(0) },
		)
		.await?;

	Ok(response.transaction)
}

fn update_governance_tx(
	multi_sig_policy: &[u8],
	version_oracle_validator: &[u8],
	version_oracle_policy: &[u8],
	genesis_utxo: UtxoId,
	new_governance_authority: MainchainAddressHash,
	tx_context: &TransactionContext,
	governance_data: &GovernanceData,
	mint_ex_units: ExUnits,
	spend_ex_units: ExUnits,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		PlutusScript::from_wrapped_cbor(multi_sig_policy, Language::new_plutus_v2())?
			.apply_uplc_data(multisig_governance_policy_configuration(new_governance_authority))?;
	let version_oracle_validator =
		PlutusScript::from_wrapped_cbor(version_oracle_validator, Language::new_plutus_v2())?
			.apply_data(genesis_utxo)?;
	let version_oracle_policy =
		PlutusScript::from_wrapped_cbor(version_oracle_policy, Language::new_plutus_v2())?
			.apply_data(genesis_utxo)?
			.apply_uplc_data(version_oracle_validator.address_data(tx_context.network)?)?;

	let config = crate::csl::get_builder_config(tx_context)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy_script,
		&governance_data.utxo_id_as_tx_input(),
		&mint_ex_units,
	)?;

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle_validator.clone(),
		version_oracle_policy.clone(),
		multi_sig_policy.clone(),
		tx_context.network,
		tx_context,
	)?)?;

	tx_builder.set_inputs(&{
		let mut inputs = TxInputsBuilder::new();
		inputs.add_script_utxo_input_with_data(
			&governance_data.utxo,
			&version_oracle_validator,
			&PlutusData::new_integer(&(raw_scripts::ScriptId::GovernancePolicy as u32).into()),
			&spend_ex_units,
		)?;

		inputs
	});

	Ok(tx_builder.balance_update_and_build(tx_context)?)
}
