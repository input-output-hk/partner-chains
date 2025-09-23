//!
//! Specification for deposit transaction:
//!
//! Consumes:
//! - UTXO at the Reserve Validator address
//!
//! Outputs:
//! - UTXO at the illiquid supply validator address with all the Reserve Tokens, plutus data Constr 0 []
//! - UTXO at the payment address with change and governance token
//!
//! Mints:
//! - Governance Token
//! - Reserve Auth Policy Token token -1 (burn)
//!
//! Reference UTOXs:
//! - Version Oracle Validator script
//! - Reserve Auth Policy script
//! - Reserve Validator script
//! - Illiquid Supply Validator script

use super::{ReserveUtxo, reserve_utxo_input_with_validator_script_reference};
use crate::{
	TokenAmount,
	await_tx::AwaitTx,
	bridge::ICSData,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		AssetIdExt, CostStore, Costs, OgmiosUtxoExt, Script, TransactionBuilderExt,
		TransactionContext, TransactionExt, TransactionOutputAmountBuilderExt, get_builder_config,
	},
	governance::GovernanceData,
	multisig::{MultiSigSmartContractResult, submit_or_create_tx_to_sign},
	reserve::ReserveData,
	scripts_data::ICSScripts,
};
use cardano_serialization_lib::*;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::{bridge::TokenTransferDatumV1, reserve::ReserveRedeemer};
use sidechain_domain::UtxoId;

/// Spends current UTXO at validator address to illiquid supply validator and burn reserve auth policy token, preventing further operations.
pub async fn handover_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = GovernanceData::get(genesis_utxo, client).await?;
	let reserve = ReserveData::get(genesis_utxo, &ctx, client).await?;
	let ics_data = ICSData::get(genesis_utxo, &ctx, client).await?;

	let ref reserve_utxo @ ReserveUtxo { ref utxo, .. } =
		reserve.get_reserve_utxo(&ctx, client).await?;
	let amount = get_amount_to_release(reserve_utxo);

	submit_or_create_tx_to_sign(
		&governance,
		ctx,
		|costs, ctx| build_tx(&amount, utxo, &reserve, &ics_data, &governance, costs, &ctx),
		"Handover Reserve",
		client,
		await_tx,
	)
	.await
}

fn get_amount_to_release(reserve_utxo: &ReserveUtxo) -> TokenAmount {
	let token = reserve_utxo.datum.immutable_settings.token.clone();
	let amount = reserve_utxo.utxo.get_asset_amount(&token);
	TokenAmount { token, amount }
}

fn build_tx(
	handover_amount: &TokenAmount,
	reserve_utxo: &OgmiosUtxo,
	reserve: &ReserveData,
	ics_data: &ICSData,
	governance: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let reserve_auth_policy_spend_cost = costs.get_one_spend();

	// mint goveranance token
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy.script(),
		&governance.utxo_id_as_tx_input(),
		&costs,
	)?;

	// Spends UTXO with Reserve Auth Policy Token and Reserve (Reward) tokens
	tx_builder.set_inputs(&reserve_utxo_input_with_validator_script_reference(
		reserve_utxo,
		reserve,
		ReserveRedeemer::Handover,
		&reserve_auth_policy_spend_cost,
	)?);

	// burn reserve auth policy token
	tx_builder.add_mint_script_token_using_reference_script(
		&Script::Plutus(reserve.scripts.auth_policy.clone()),
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		&Int::new_i32(-1),
		&costs,
	)?;

	tx_builder.add_output(&illiquid_supply_validator_output(
		handover_amount,
		&ics_data.scripts,
		ctx,
	)?)?;
	// This reference input is needed by the Reserve Validator with 'Handover' redeemer.
	tx_builder.add_script_reference_input(
		&ics_data.validator_version_utxo.to_csl_tx_input(),
		ics_data.scripts.validator.bytes.len(),
	);
	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

// Creates output with reserve token and updated deposit
fn illiquid_supply_validator_output(
	output_value: &TokenAmount,
	scripts: &ICSScripts,
	ctx: &TransactionContext,
) -> Result<TransactionOutput, JsError> {
	let tx_output_builder =
		TransactionOutputBuilder::new().with_address(&scripts.validator.address(ctx.network));
	if output_value.amount > 0 {
		let ma = output_value.token.to_multi_asset(output_value.amount)?;
		let amount_builder = tx_output_builder
			.with_plutus_data(&TokenTransferDatumV1::ReserveTransfer.into())
			.next()?;
		amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()
	} else {
		// Smart-contract requires to deposit exactly one UTXO in the illiquid supply validator,
		// otherwise it returns ERROR-RESERVE-16: No unique output utxo at the illiquid circulation supply address
		let amount_builder = tx_output_builder.next()?;
		amount_builder.with_minimum_ada(ctx)?.build()
	}
}
