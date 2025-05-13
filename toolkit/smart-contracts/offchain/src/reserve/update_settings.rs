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

use super::{ReserveData, reserve_utxo_input_with_validator_script_reference};
use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::*,
	governance::GovernanceData,
	multisig::{MultiSigSmartContractResult, submit_or_create_tx_to_sign},
	reserve::ReserveUtxo,
};
use cardano_serialization_lib::*;
use ogmios_client::types::OgmiosUtxo;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use partner_chains_plutus_data::reserve::{ReserveDatum, ReserveRedeemer};
use sidechain_domain::{ScriptHash, UtxoId};

pub async fn update_reserve_settings<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	total_accrued_function_script_hash: ScriptHash,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = GovernanceData::get(genesis_utxo, client).await?;
	let reserve = ReserveData::get(genesis_utxo, &ctx, client).await?;
	let ReserveUtxo { utxo: reserve_utxo, datum: mut reserve_datum } =
		reserve.get_reserve_utxo(&ctx, client).await?;

	if total_accrued_function_script_hash
		== reserve_datum.mutable_settings.total_accrued_function_asset_name
	{
		log::info!(
			"Reserve V function hash is already set to {:?}. Nothing to update.",
			total_accrued_function_script_hash
		);
		return Ok(None);
	}
	reserve_datum.mutable_settings.total_accrued_function_asset_name =
		total_accrued_function_script_hash.clone();

	Ok(Some(
		submit_or_create_tx_to_sign(
			&governance,
			ctx,
			|costs, ctx| {
				update_reserve_settings_tx(
					&reserve_datum,
					&reserve,
					&governance,
					&reserve_utxo,
					costs,
					&ctx,
				)
			},
			"Update Reserve Settings",
			client,
			await_tx,
		)
		.await?,
	))
}

fn update_reserve_settings_tx(
	datum: &ReserveDatum,
	reserve: &ReserveData,
	governance: &GovernanceData,
	reserve_utxo: &OgmiosUtxo,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	// spend old settings
	tx_builder.set_inputs(&reserve_utxo_input_with_validator_script_reference(
		reserve_utxo,
		reserve,
		ReserveRedeemer::UpdateReserve,
		&costs.get_one_spend(),
	)?);
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

	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy.script(),
		&governance.utxo_id_as_tx_input(),
		&costs,
	)?;
	tx_builder.add_script_reference_input(
		&reserve.illiquid_circulation_supply_validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.illiquid_circulation_supply_validator.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		reserve.scripts.auth_policy.bytes.len(),
	);
	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}
