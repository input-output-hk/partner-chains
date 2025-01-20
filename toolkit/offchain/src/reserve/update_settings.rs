//! Transaction that updates reserve settings.
//!
//! Specification:
//! 1. The transaction should mint one token:
//!   * 1 Governance Policy Token (using reference script)
//! 2. The transaction should spend one token:
//!   * 1 Reserve Auth Policy Token (using reference script)
//! 3. The transaction should have two outputs:
//!   * Reserve Validator output that:
//!   * * has value from existing reserve UTXO
//!   * * has the updated Plutus Data (in our "versioned format"): `[[[Int(t0), <Encoded Token>], [Bytes(v_function_hash), Int(initial_incentive)], [Int(0)]], Constr(0, []), Int(0)]`,
//!       where `<Encoded Token>` is `Constr(0, [Bytes(policy_id), Bytes(asset_name)])`.
//!   * Change output that keeps the Governance Token and change of other tokens
//! 4. The transaction should have three script reference inputs:
//!   * Reserve Auth Version Utxo
//!   * Reserve Validator Version Utxo
//!   * Governance Policy Script

use super::ReserveData;
use crate::reserve::ReserveUtxo;
use crate::{
	await_tx::AwaitTx, csl::*, init_governance::get_governance_data,
	init_governance::GovernanceData,
};
use cardano_serialization_lib::*;
use ogmios_client::types::OgmiosUtxo;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use partner_chains_plutus_data::reserve::{ReserveDatum, ReserveRedeemer};
use sidechain_domain::{McTxHash, ScriptHash, UtxoId};

pub async fn update_reserve_settings<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	mut total_accrued_function_script_hash_opt: Option<ScriptHash>,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = get_governance_data(genesis_utxo, client).await?;
	let reserve = ReserveData::get(genesis_utxo, &ctx, client).await?;
	let ReserveUtxo { reserve_utxo, mut reserve_settings } =
		reserve.get_reserve_utxo(&ctx, client).await?;

	if let Some(total_accrued_function_script_hash) = total_accrued_function_script_hash_opt.clone()
	{
		if total_accrued_function_script_hash
			== reserve_settings.mutable_settings.total_accrued_function_script_hash
		{
			total_accrued_function_script_hash_opt = None;
			log::info!(
				"Reserve V function hash is already set to {:?}.",
				total_accrued_function_script_hash
			);
		} else {
			reserve_settings.mutable_settings.total_accrued_function_script_hash =
				total_accrued_function_script_hash.clone();
		}
	}

	if total_accrued_function_script_hash_opt.is_none() {
		log::info!("Nothing to update.");
		return Ok(None);
	}

	let tx_to_evaluate = update_reserve_settings_tx(
		&reserve_settings,
		&reserve,
		&governance,
		&zero_ex_units(),
		&reserve_utxo,
		&zero_ex_units(),
		&ctx,
	)?;

	let evaluate_response = client.evaluate_transaction(&tx_to_evaluate.to_bytes()).await?;

	let budgets = get_validator_budgets(evaluate_response);
	let gov_cost = budgets
		.mint_ex_units
		.first()
		.expect("Every Update Reserve Settings transaction should have one mint")
		.clone();
	let reserve_cost = budgets
		.spend_ex_units
		.first()
		.expect("Every Update Reserve Settings transaction should have one spend")
		.clone();

	let tx = update_reserve_settings_tx(
		&reserve_settings,
		&reserve,
		&governance,
		&gov_cost,
		&reserve_utxo,
		&reserve_cost,
		&ctx,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Update Reserve Settings transaction request failed: {}, tx bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!("Update Reserve Settings transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;
	Ok(Some(McTxHash(tx_id)))
}

fn update_reserve_settings_tx(
	datum: &ReserveDatum,
	reserve: &ReserveData,
	governance: &GovernanceData,
	governance_script_cost: &ExUnits,
	reserve_utxo: &OgmiosUtxo,
	reserve_script_cost: &ExUnits,
	ctx: &TransactionContext,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	// spend old settings
	{
		let mut inputs = TxInputsBuilder::new();

		let witness = PlutusWitness::new_with_ref_without_datum(
			&PlutusScriptSource::new_ref_input(
				&reserve.scripts.validator.csl_script_hash(),
				&reserve.validator_version_utxo.to_csl_tx_input(),
				&reserve.scripts.validator.language,
				reserve.scripts.validator.bytes.len(),
			),
			&Redeemer::new(
				&RedeemerTag::new_spend(),
				// CSL will set redeemer index for the index of script input after sorting transaction inputs
				&0u32.into(),
				&ReserveRedeemer::UpdateReserve { governance_version: 1u64 }.into(),
				reserve_script_cost,
			),
		);
		let amount = reserve_utxo.value.to_csl()?;
		inputs.add_plutus_script_input(&witness, &reserve_utxo.to_csl_tx_input(), &amount);

		tx_builder.set_inputs(&inputs);
	}
	{
		let amount_builder = TransactionOutputBuilder::new()
			.with_address(&reserve.scripts.validator.address(ctx.network))
			.with_plutus_data(&(datum.clone().into()))
			.next()?;
		let mut val = reserve_utxo.value.to_csl()?;
		let output = amount_builder.with_value(&val).build()?;
		let min_ada = MinOutputAdaCalculator::new(
			&output,
			&DataCost::new_coins_per_byte(
				&ctx.protocol_parameters.min_utxo_deposit_coefficient.into(),
			),
		)
		.calculate_ada()?;
		val.set_coin(&min_ada);
		let a = amount_builder.with_value(&val).build()?;
		tx_builder.add_output(&a)?;
	}

	let gov_tx_input = governance.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy_script,
		&gov_tx_input,
		governance_script_cost,
	)?;
	tx_builder.add_script_reference_input(
		&reserve.illiquid_circulation_supply_validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.illiquid_circulation_supply_validator.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		reserve.scripts.auth_policy.bytes.len(),
	);
	tx_builder.balance_update_and_build(ctx)
}
