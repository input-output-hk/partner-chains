//! Transaction that creates a new reserve.
//!
//! Specification:
//! 1. The transaction should mint two tokens:
//!   * 1 Reserve Auth Policy Token (using reference script)
//!   * 1 Governance Policy Token (using reference script)
//! 2. The transaction should have two outputs:
//!   * Reserve Validator output that:
//!   * * has Reward Tokens and minted Reserve Auth Policy Token
//!   * * has Plutus Data (in our "versioned format"): `[[[Int(t0), <Encoded Token>], [Bytes(v_function_hash), Int(initial_incentive)], [Int(0)]], Constr(0, []), Int(0)]`,
//!       where `<Encoded Token>` is `Constr(0, [Bytes(policy_id), Bytes(asset_name)])`.
//!   * Change output that keeps the Governance Token and change of other tokens
//! 3. The transaction should have three script reference inputs:
//!   * Reserve Auth Version Utxo
//!   * Reserve Validator Version Utxo
//!   * Governance Policy Script

use super::ReserveData;
use crate::{
	await_tx::AwaitTx,
	csl::{
		empty_asset_name, get_builder_config, get_validator_budgets, zero_ex_units, AssetNameExt,
		OgmiosUtxoExt, TransactionBuilderExt, TransactionContext,
		TransactionOutputAmountBuilderExt,
	},
	init_governance::{get_governance_data, GovernanceData},
	scripts_data::ReserveScripts,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	Assets, ExUnits, JsError, MultiAsset, Transaction, TransactionBuilder, TransactionOutput,
	TransactionOutputBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::{OgmiosEvaluateTransactionResponse, Transactions},
};
use partner_chains_plutus_data::reserve::{
	ReserveDatum, ReserveImmutableSettings, ReserveMutableSettings, ReserveStats,
};
use sidechain_domain::{AssetId, McTxHash, PolicyId, UtxoId};
use std::collections::HashMap;

pub async fn create_reserve_utxo<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	parameters: ReserveParameters,
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	client: &T,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = get_governance_data(genesis_utxo, client).await?;
	let reserve = ReserveData::get(genesis_utxo, &ctx, client).await?;

	let tx_to_evaluate = create_reserve_tx(
		&parameters,
		&reserve,
		&governance,
		zero_ex_units(),
		zero_ex_units(),
		&ctx,
	)?;
	let evaluate_response = client.evaluate_transaction(&tx_to_evaluate.to_bytes()).await?;

	let (reserve_auth_ex_units, governance_ex_units) = match_costs(
		&tx_to_evaluate,
		&reserve.scripts.auth_policy.csl_script_hash(),
		&governance.policy_script_hash(),
		evaluate_response,
	)?;

	let tx = create_reserve_tx(
		&parameters,
		&reserve,
		&governance,
		governance_ex_units,
		reserve_auth_ex_units,
		&ctx,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Create Reserve transaction request failed: {}, tx bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!("Create Reserve transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;
	Ok(McTxHash(tx_id))
}

pub struct ReserveParameters {
	pub initial_incentive: u64,
	pub total_accrued_function_script_hash: PolicyId,
	pub token: AssetId,
	pub initial_deposit: u64,
}

impl From<&ReserveParameters> for ReserveDatum {
	fn from(value: &ReserveParameters) -> Self {
		ReserveDatum {
			immutable_settings: ReserveImmutableSettings { token: value.token.clone() },
			mutable_settings: ReserveMutableSettings {
				total_accrued_function_script_hash: value
					.total_accrued_function_script_hash
					.clone(),
				initial_incentive: value.initial_incentive,
			},
			stats: ReserveStats { token_total_amount_transferred: 0 },
		}
	}
}

fn create_reserve_tx(
	parameters: &ReserveParameters,
	reserve: &ReserveData,
	governance: &GovernanceData,
	governance_script_cost: ExUnits,
	reserve_auth_script_cost: ExUnits,
	ctx: &TransactionContext,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&reserve.scripts.auth_policy,
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		&reserve_auth_script_cost,
	)?;
	tx_builder.add_output(&reserve_validator_output(parameters, &reserve.scripts, ctx)?)?;

	let gov_tx_input = governance.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy_script,
		&gov_tx_input,
		&governance_script_cost,
	)?;
	tx_builder.add_script_reference_input(
		&reserve.validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.validator.bytes.len(),
	);
	tx_builder.add_required_signer(&ctx.payment_key_hash());
	tx_builder.balance_update_and_build(ctx)
}

fn match_costs(
	evaluated_transaction: &Transaction,
	reserve_auth_policy: &cardano_serialization_lib::ScriptHash,
	governance_policy: &cardano_serialization_lib::ScriptHash,
	evaluate_response: Vec<OgmiosEvaluateTransactionResponse>,
) -> Result<(ExUnits, ExUnits), anyhow::Error> {
	let mint_keys = evaluated_transaction
		.body()
		.mint()
		.expect("Every Create Reserve transaction should have two mints")
		.keys();
	let script_to_index: HashMap<cardano_serialization_lib::ScriptHash, usize> =
		vec![(mint_keys.get(0), 0), (mint_keys.get(1), 1)].into_iter().collect();
	let mint_ex_units = get_validator_budgets(evaluate_response).mint_ex_units;
	if mint_ex_units.len() == 2 {
		let reserve_auth_policy_idx = *script_to_index
			.get(reserve_auth_policy)
			.expect("Reserve Auth Policy Token is present in transaction mints");
		let reserve_auth_ex_units = mint_ex_units
			.get(reserve_auth_policy_idx)
			.expect("mint_ex_units have two items")
			.clone();
		let gov_policy_idx = *script_to_index
			.get(governance_policy)
			.expect("Governance Policy Token is present in transaction mints");
		let governance_ex_units =
			mint_ex_units.get(gov_policy_idx).expect("mint_ex_units have two items").clone();
		Ok((reserve_auth_ex_units, governance_ex_units))
	} else {
		Err(anyhow!("Could not build transaction to submit, evaluate response has wrong number of mint keys."))
	}
}

// Creates output with reserve token and the initial deposit
fn reserve_validator_output(
	parameters: &ReserveParameters,
	scripts: &ReserveScripts,
	ctx: &TransactionContext,
) -> Result<TransactionOutput, JsError> {
	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&scripts.validator.address(ctx.network))
		.with_plutus_data(&ReserveDatum::from(parameters).into())
		.next()?;
	let mut ma = MultiAsset::new();
	let mut assets = Assets::new();
	assets.insert(&empty_asset_name(), &1u64.into());
	ma.insert(&scripts.auth_policy.csl_script_hash(), &assets);

	let AssetId { policy_id, asset_name } = parameters.token.clone();
	let mut assets = Assets::new();
	assets.insert(
		&asset_name.to_csl().expect("AssetName has a valid length"),
		&parameters.initial_deposit.into(),
	);
	ma.insert(&policy_id.0.into(), &assets);

	amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()
}
