use crate::{
	await_tx::AwaitTx,
	bridge::ICSData,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		Costs, MultiAssetExt, OgmiosUtxoExt, Script, TransactionBuilderExt, TransactionContext,
		TransactionExt, TransactionOutputAmountBuilderExt, get_builder_config, unit_plutus_data,
	},
	governance::GovernanceData,
	multisig::{MultiSigSmartContractResult, submit_or_create_tx_to_sign},
	scripts_data::ICSScripts,
};
use cardano_serialization_lib::{
	Int, JsError, MultiAsset, Transaction, TransactionBuilder, TransactionOutput,
	TransactionOutputBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use sidechain_domain::UtxoId;

/// Creates "blessed" UTXOs at the ICS (Bridge) validator.
pub async fn create_validator_utxos<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	amount: u64,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult> {
	let payment_ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = GovernanceData::get(genesis_utxo, client).await?;
	let ics_data = ICSData::get(genesis_utxo, &payment_ctx, client).await?;

	submit_or_create_tx_to_sign(
		&governance,
		payment_ctx,
		|costs, ctx| create_reserve_tx(amount, &ics_data, &governance, costs, &ctx),
		"Create Bridge UTXOs",
		client,
		await_tx,
	)
	.await
}

fn create_reserve_tx(
	amount: u64,
	ics_data: &ICSData,
	governance: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let gov_tx_input = governance.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	tx_builder.add_mint_script_token_using_reference_script(
		&Script::Plutus(ics_data.scripts.auth_policy.clone()),
		&ics_data.auth_policy_version_utxo.to_csl_tx_input(),
		&Int::new(&amount.into()),
		&costs,
	)?;
	// Create ICS Authorized Outputs. These contain special ICS Authority Token,
	// that prevents UTxOs from being merged all into one.
	for _ in 0u64..amount {
		tx_builder.add_output(&ics_validator_output(&ics_data.scripts, ctx)?)?;
	}

	let tx = tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses();

	Ok(tx)
}

fn ics_validator_output(
	scripts: &ICSScripts,
	ctx: &TransactionContext,
) -> Result<TransactionOutput, JsError> {
	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&scripts.validator.address(ctx.network))
		.with_plutus_data(&unit_plutus_data())
		.next()?;
	let ma = MultiAsset::new().with_asset_amount(&scripts.auth_policy.empty_name_asset(), 1u64)?;

	amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()
}
