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

use super::{reserve_utxo_input_with_validator_script_reference, ReserveUtxo, TokenAmount};
use crate::{
	await_tx::AwaitTx,
	csl::{
		get_builder_config, AssetIdExt, CostStore, Costs, OgmiosUtxoExt, TransactionBuilderExt,
		TransactionContext, TransactionOutputAmountBuilderExt,
	},
	init_governance::{get_governance_data, GovernanceData},
	reserve::ReserveData,
	scripts_data::ReserveScripts,
};
use anyhow::anyhow;
use cardano_serialization_lib::*;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::reserve::ReserveRedeemer;
use sidechain_domain::{McTxHash, UtxoId};

/// Spends current UTXO at validator address to illiquid supply validator and burn reserve auth policy token, preventing further operations.
pub async fn handover_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	client: &T,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = get_governance_data(genesis_utxo, client).await?;
	let reserve = ReserveData::get(genesis_utxo, &ctx, client).await?;

	let reserve_utxo = reserve.get_reserve_utxo(&ctx, client).await?;
	let amount = get_amount_to_release(&reserve_utxo).ok_or_else(|| anyhow!("Internal Error. Reserve Validator has UTXO with Reserve Auth Policy Token, but without other asset."))?;
	let utxo = reserve_utxo.reserve_utxo;

	let tx = Costs::calculate_costs(
		|costs| build_tx(&amount, &utxo, &reserve, &governance, costs, &ctx),
		client,
	)
	.await?;

	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Handover Reserve transaction request failed: {}, tx bytes: {}",
			e,
			hex::encode(&signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!("Handover Reserve transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;
	Ok(McTxHash(tx_id))
}

fn get_amount_to_release(reserve_utxo: &ReserveUtxo) -> Option<TokenAmount> {
	let token = reserve_utxo.reserve_settings.immutable_settings.token.clone();
	let amount = reserve_utxo.reserve_utxo.get_asset_amount(&token).try_into().ok()?;
	Some(TokenAmount { token, amount })
}

fn build_tx(
	handover_amount: &TokenAmount,
	reserve_utxo: &OgmiosUtxo,
	reserve: &ReserveData,
	governance: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let reserve_auth_policy_spend_cost = costs.get_one_spend();
	let reserve_auth_policy_burn_cost = costs.get_mint(&reserve.scripts.auth_policy);
	let governance_mint_cost = costs.get_mint(&governance.policy_script);

	// mint goveranance token
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy_script,
		&governance.utxo_id_as_tx_input(),
		&governance_mint_cost,
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
		&reserve.scripts.auth_policy,
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		&Int::new_i32(-1),
		&reserve_auth_policy_burn_cost,
	)?;

	tx_builder.add_output(&illiquid_supply_validator_output(
		handover_amount,
		&reserve.scripts,
		ctx,
	)?)?;
	tx_builder.add_script_reference_input(
		&reserve.illiquid_circulation_supply_validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.illiquid_circulation_supply_validator.bytes.len(),
	);
	tx_builder.balance_update_and_build(ctx)
}

// Creates output with reserve token and updated deposit
fn illiquid_supply_validator_output(
	output_value: &TokenAmount,
	scripts: &ReserveScripts,
	ctx: &TransactionContext,
) -> Result<TransactionOutput, JsError> {
	let ma = output_value.token.to_multi_asset(output_value.amount)?;
	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&scripts.illiquid_circulation_supply_validator.address(ctx.network))
		.with_plutus_data(&illiquid_supply_validator_redeemer())
		.next()?;
	amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()
}

fn illiquid_supply_validator_redeemer() -> PlutusData {
	PlutusData::new_empty_constr_plutus_data(&BigNum::zero())
}
