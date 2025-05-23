//!
//! Specification for deposit transaction:
//!
//! Consumes:
//! - UTXO at the validator address
//! - UTXOs at payment address that have tokens to be deposited
//!
//! Produces:
//! - UTXO at the validator address with increased token amount
//! - UTXO at the payment address with change
//!
//! Reference UTOXs:
//! - Version Oracle Validator script
//! - Reserve Auth Policy script
//! - Reserve Validator script
//! - Illiquid Supply Validator script

use super::{ReserveData, TokenAmount, reserve_utxo_input_with_validator_script_reference};
use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		CostStore, Costs, MultiAssetExt, OgmiosUtxoExt, TransactionBuilderExt, TransactionContext,
		TransactionExt, TransactionOutputAmountBuilderExt, get_builder_config,
	},
	governance::GovernanceData,
	multisig::{MultiSigSmartContractResult, submit_or_create_tx_to_sign},
	scripts_data::ReserveScripts,
};
use cardano_serialization_lib::{
	JsError, MultiAsset, Transaction, TransactionBuilder, TransactionOutput,
	TransactionOutputBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::reserve::ReserveRedeemer;
use sidechain_domain::{AssetId, UtxoId};

/// Spends current UTXO at validator address and creates a new UTXO with increased token amount
pub async fn deposit_to_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	amount: u64,
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult> {
	let payment_context = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = GovernanceData::get(genesis_utxo, client).await?;
	let reserve = ReserveData::get(genesis_utxo, &payment_context, client).await?;
	let reserve_utxo = reserve.get_reserve_utxo(&payment_context, client).await?;
	let utxo = reserve_utxo.utxo;
	let token = reserve_utxo.datum.immutable_settings.token;
	let current_amount = get_token_amount(&utxo, &token);
	let token_amount = TokenAmount { token, amount: current_amount + amount };

	submit_or_create_tx_to_sign(
		&governance,
		payment_context,
		|costs, ctx| {
			deposit_to_reserve_tx(&token_amount, &utxo, &reserve, &governance, costs, &ctx)
		},
		"Deposit to Reserve",
		client,
		await_tx,
	)
	.await
}

fn get_token_amount(utxo: &OgmiosUtxo, token: &AssetId) -> u64 {
	let AssetId { policy_id, asset_name } = token;
	utxo.value
		.native_tokens
		.get(&policy_id.0)
		.and_then(|assets| assets.iter().find(|asset| asset.name == asset_name.0.to_vec()))
		.map(|asset| asset.amount)
		.unwrap_or(0) // Token can be not found if the reserve was created with the initial deposit of 0 tokens
}

fn deposit_to_reserve_tx(
	parameters: &TokenAmount,
	current_utxo: &OgmiosUtxo,
	reserve: &ReserveData,
	governance: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	tx_builder.add_output(&validator_output(parameters, current_utxo, &reserve.scripts, ctx)?)?;

	tx_builder.set_inputs(&reserve_utxo_input_with_validator_script_reference(
		current_utxo,
		reserve,
		ReserveRedeemer::DepositToReserve,
		&costs.get_one_spend(),
	)?);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy.script(),
		&governance.utxo_id_as_tx_input(),
		&costs,
	)?;

	tx_builder.add_script_reference_input(
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		reserve.scripts.auth_policy.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&reserve.illiquid_circulation_supply_validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.illiquid_circulation_supply_validator.bytes.len(),
	);
	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

// Creates output with reserve token and updated deposit
fn validator_output(
	token_amount: &TokenAmount,
	current_utxo: &OgmiosUtxo,
	scripts: &ReserveScripts,
	ctx: &TransactionContext,
) -> Result<TransactionOutput, JsError> {
	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&scripts.validator.address(ctx.network))
		.with_plutus_data(
			&current_utxo
				.get_plutus_data()
				.expect("Current UTXO datum was parsed hence it exists"),
		)
		.next()?;
	let ma = MultiAsset::new()
		.with_asset_amount(&token_amount.token, token_amount.amount)?
		.with_asset_amount(&scripts.auth_policy.empty_name_asset(), 1u64)?;

	amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()
}
