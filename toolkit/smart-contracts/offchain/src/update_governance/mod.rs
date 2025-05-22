//! Building and submitting of the transaction that changes the current governance authority
//!
//! The transaction:
//! 1. Burns the previous governance UTXO from the version oracle validator address
//! 2. Mints exactly 1 multi-sig policy token as authentication
//! 3. Produces a new governance UTXO at the version oracle validator address with a version oracle
//!    Plutus datum attached that contains the script ID (32) and policy hash.
use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		CostStore, Costs, InputsBuilderExt, TransactionBuilderExt, TransactionContext,
		TransactionExt,
	},
	governance::{GovernanceData, MultiSigParameters},
	init_governance::transaction::version_oracle_datum_output,
	multisig::{MultiSigSmartContractResult, submit_or_create_tx_to_sign},
	plutus_script,
};
use cardano_serialization_lib::{PlutusData, Transaction, TransactionBuilder, TxInputsBuilder};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use raw_scripts::RawScript;
use sidechain_domain::UtxoId;

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_values;

/// Updates governance parameters.
pub async fn run_update_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	governance_parameters: &MultiSigParameters,
	payment_key: &CardanoPaymentSigningKey,
	genesis_utxo_id: UtxoId,
	client: &T,
	await_tx: A,
) -> anyhow::Result<MultiSigSmartContractResult> {
	let payment_ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance_data = GovernanceData::get(genesis_utxo_id, client).await?;

	submit_or_create_tx_to_sign(
		&governance_data,
		payment_ctx,
		|costs, ctx| {
			update_governance_tx(
				raw_scripts::VERSION_ORACLE_VALIDATOR,
				raw_scripts::VERSION_ORACLE_POLICY,
				genesis_utxo_id,
				governance_parameters,
				&governance_data,
				costs,
				ctx,
			)
		},
		"Update Governance",
		client,
		&await_tx,
	)
	.await
}

fn update_governance_tx(
	version_oracle_validator: RawScript,
	version_oracle_policy: RawScript,
	genesis_utxo: UtxoId,
	governance_parameters: &MultiSigParameters,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy = governance_parameters.as_simple_at_least_n().to_csl_native_script();
	let version_oracle_validator = plutus_script![version_oracle_validator, genesis_utxo]?;
	let version_oracle_policy = plutus_script![
		version_oracle_policy,
		genesis_utxo,
		version_oracle_validator.address_data(ctx.network)?
	]?;

	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy.script(),
		&governance_data.utxo_id_as_tx_input(),
		&costs,
	)?;

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle_validator.clone(),
		version_oracle_policy.clone(),
		multi_sig_policy,
		ctx.network,
		ctx,
	)?)?;

	tx_builder.set_inputs(&{
		let mut inputs = TxInputsBuilder::new();
		inputs.add_script_utxo_input(
			&governance_data.utxo,
			&version_oracle_validator,
			&PlutusData::new_integer(&(raw_scripts::ScriptId::GovernancePolicy as u32).into()),
			&costs.get_one_spend(),
		)?;

		inputs
	});

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}
