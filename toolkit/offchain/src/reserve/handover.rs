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

use super::{ReserveUtxo, TokenAmount};
use crate::{
	await_tx::AwaitTx,
	csl::{
		get_builder_config, get_validator_budgets, zero_ex_units, AssetIdExt, OgmiosUtxoExt,
		OgmiosValueExt, ScriptExUnits, TransactionBuilderExt, TransactionContext,
		TransactionOutputAmountBuilderExt,
	},
	init_governance::{get_governance_data, GovernanceData},
	plutus_script::PlutusScript,
	reserve::ReserveData,
	scripts_data::ReserveScripts,
};
use anyhow::anyhow;
use cardano_serialization_lib::*;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::{OgmiosEvaluateTransactionResponse, Transactions},
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

	let tx_to_evaluate =
		build_tx(&amount, &utxo, &reserve, &governance, ScriptsCosts::zero(), &ctx)?;
	let evaluate_response = client.evaluate_transaction(&tx_to_evaluate.to_bytes()).await?;
	let script_costs = ScriptsCosts::from_ogmios(
		evaluate_response,
		&reserve.scripts.auth_policy,
		&governance.policy_script,
	)?;

	// ETCM-9222 - this transaction manifests problem that input selection after the first evaluation can affects the cost of the transaction.
	let tx = build_tx(&amount, &utxo, &reserve, &governance, script_costs, &ctx)?;
	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await?;
	let script_costs = ScriptsCosts::from_ogmios(
		evaluate_response,
		&reserve.scripts.auth_policy,
		&governance.policy_script,
	)?;

	let tx = build_tx(&amount, &utxo, &reserve, &governance, script_costs, &ctx)?;

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
	costs: ScriptsCosts,
	ctx: &TransactionContext,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	// mint goveranance token
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy_script,
		&governance.utxo_id_as_tx_input(),
		&costs.governance_mint,
	)?;

	let inputs = reserve_token_input(reserve_utxo, reserve, costs.reserve_auth_policy_spend)?;
	tx_builder.set_inputs(&inputs);

	// burn reserve auth policy token
	tx_builder.add_mint_script_token_using_reference_script(
		&reserve.scripts.auth_policy,
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		&Int::new_i32(-1),
		&costs.reserve_auth_policy_burn,
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

/// Spends UTXO with Reserve Auth Policy Token and Reserve (Reward) tokens
fn reserve_token_input(
	reserve_auth_token_utxo: &OgmiosUtxo,
	reserve: &ReserveData,
	cost: ExUnits,
) -> Result<TxInputsBuilder, JsError> {
	let mut inputs = TxInputsBuilder::new();
	let input = reserve_auth_token_utxo.to_csl_tx_input();
	let amount = reserve_auth_token_utxo.value.to_csl()?;

	let validator_version_utxo_id = reserve.validator_version_utxo.to_csl_tx_input();
	let validator = &reserve.scripts.validator;

	let redeemer_data = ReserveRedeemer::Handover { governance_version: 1 }.into();
	let witness = PlutusWitness::new_with_ref_without_datum(
		&PlutusScriptSource::new_ref_input(
			&validator.csl_script_hash(),
			&validator_version_utxo_id,
			&validator.language,
			validator.bytes.len(),
		),
		&Redeemer::new(&RedeemerTag::new_spend(), &0u32.into(), &redeemer_data, &cost),
	);
	inputs.add_plutus_script_input(&witness, &input, &amount);

	Ok(inputs)
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

struct ScriptsCosts {
	reserve_auth_policy_spend: ExUnits,
	reserve_auth_policy_burn: ExUnits,
	governance_mint: ExUnits,
}

impl ScriptsCosts {
	fn zero() -> Self {
		Self {
			reserve_auth_policy_spend: zero_ex_units(),
			reserve_auth_policy_burn: zero_ex_units(),
			governance_mint: zero_ex_units(),
		}
	}

	fn from_ogmios(
		response: Vec<OgmiosEvaluateTransactionResponse>,
		reserve_auth_policy_script: &PlutusScript,
		governance_policy_script: &PlutusScript,
	) -> Result<Self, anyhow::Error> {
		let ScriptExUnits { mut mint_ex_units, mut spend_ex_units } =
			get_validator_budgets(response);
		let reserve_auth_policy_spend = spend_ex_units
			.pop()
			.ok_or_else(|| anyhow!("Evaluate response does not have expected 'spend' cost"))?;
		let mint_1 = mint_ex_units
			.pop()
			.ok_or_else(|| anyhow!("Evaluate response does not have expected 'mint' costs"))?;
		let mint_0 = mint_ex_units
			.pop()
			.ok_or_else(|| anyhow!("Evaluate response does not have expected 'mint' costs"))?;
		let (reserve_auth_policy_mint, governance_mint) =
			if reserve_auth_policy_script.script_hash() < governance_policy_script.script_hash() {
				(mint_0, mint_1)
			} else {
				(mint_1, mint_0)
			};
		Ok(Self {
			reserve_auth_policy_spend,
			reserve_auth_policy_burn: reserve_auth_policy_mint,
			governance_mint,
		})
	}
}
