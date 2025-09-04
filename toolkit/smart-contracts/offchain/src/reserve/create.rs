//! Transaction that creates a new reserve.
//!
//! Specification:
//! 1. The transaction should mint two tokens:
//!   * 1 Reserve Auth Policy Token (using reference script)
//!   * 1 Governance Policy Token (using reference script)
//! 2. The transaction should have two outputs:
//!   * Reserve Validator output that:
//!   * * has Reward Tokens and minted Reserve Auth Policy Token
//!   * * has Plutus Data (in our "versioned format"): `[[[<Encoded Token>], [Bytes(v_function_hash), Int(initial_incentive)], [Int(0)]], Constr(0, []), Int(0)]`,
//!       where `<Encoded Token>` is `Constr(0, [Bytes(policy_id), Bytes(asset_name)])`.
//!   * Change output that keeps the Governance Token and change of other tokens
//! 3. The transaction should have three script reference inputs:
//!   * Reserve Auth Version Utxo
//!   * Reserve Validator Version Utxo
//!   * Governance Policy Script

use super::ReserveData;
use crate::csl::unit_plutus_data;
use crate::reserve::create::Script::Plutus;
use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		Costs, MultiAssetExt, OgmiosUtxoExt, Script, TransactionBuilderExt, TransactionContext,
		TransactionExt, TransactionOutputAmountBuilderExt, get_builder_config,
	},
	governance::GovernanceData,
	multisig::{MultiSigSmartContractResult, submit_or_create_tx_to_sign},
	scripts_data::ReserveScripts,
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
use partner_chains_plutus_data::reserve::{
	ReserveDatum, ReserveImmutableSettings, ReserveMutableSettings, ReserveStats,
};
use sidechain_domain::{AssetId, PolicyId, UtxoId};

/// Creates new reserve with the given [ReserveParameters].
pub async fn create_reserve_utxo<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	parameters: ReserveParameters,
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult> {
	let payment_ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = GovernanceData::get(genesis_utxo, client).await?;
	let reserve = ReserveData::get(genesis_utxo, &payment_ctx, client).await?;

	submit_or_create_tx_to_sign(
		&governance,
		payment_ctx,
		|costs, ctx| create_reserve_tx(&parameters, &reserve, &governance, costs, &ctx),
		"Create Reserve",
		client,
		await_tx,
	)
	.await
}

/// Parameters for token reserve.
pub struct ReserveParameters {
	/// [PolicyId] of the V-function minting policy.
	pub total_accrued_function_script_hash: PolicyId,
	/// [AssetId] of reserve token.
	pub token: AssetId,
	/// Initial deposit amount.
	pub initial_deposit: u64,
	/// Amount of illiquid circulation supply authority tokens to mint.
	pub ics_auth_token_amount: u64,
}

impl From<&ReserveParameters> for ReserveDatum {
	fn from(value: &ReserveParameters) -> Self {
		ReserveDatum {
			immutable_settings: ReserveImmutableSettings { token: value.token.clone() },
			mutable_settings: ReserveMutableSettings {
				total_accrued_function_asset_name: value.total_accrued_function_script_hash.clone(),
				// this value is hard-coded to zero as a temporary fix because of a vulnerability in the on-chain
				// contract code that would allow the reserve to be drained for non-zero values
				initial_incentive: 0,
			},
			stats: ReserveStats { token_total_amount_transferred: 0 },
		}
	}
}

fn create_reserve_tx(
	parameters: &ReserveParameters,
	reserve: &ReserveData,
	governance: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&Script::Plutus(reserve.scripts.auth_policy.clone()),
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		&costs,
	)?;
	tx_builder.add_output(&reserve_validator_output(parameters, &reserve.scripts, ctx)?)?;

	let gov_tx_input = governance.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy.script(),
		&gov_tx_input,
		&costs,
	)?;
	tx_builder.add_script_reference_input(
		&reserve.validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.validator.bytes.len(),
	);

	if parameters.ics_auth_token_amount > 0 {
		tx_builder.add_mint_script_token_using_reference_script(
			&Plutus(reserve.scripts.illiquid_circulation_supply_auth_token_policy.clone()),
			&reserve
				.illiquid_circulation_supply_authority_token_policy_version_utxo
				.to_csl_tx_input(),
			&Int::new(&parameters.ics_auth_token_amount.into()),
			&costs,
		)?;
		// Create ICS Authorized Outputs. These contain special ICS Authority Token,
		// that prevents UTxOs from being merged all into one.
		for _ in 0..parameters.ics_auth_token_amount {
			tx_builder.add_output(&ics_validator_output(&reserve.scripts, ctx)?)?;
		}
	}

	let tx = tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses();

	Ok(tx)
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
	let ma = MultiAsset::new()
		.with_asset_amount(&scripts.auth_policy.empty_name_asset(), 1u64)?
		.with_asset_amount(&parameters.token, parameters.initial_deposit)?;

	amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()
}

fn ics_validator_output(
	scripts: &ReserveScripts,
	ctx: &TransactionContext,
) -> Result<TransactionOutput, JsError> {
	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&scripts.illiquid_circulation_supply_validator.address(ctx.network))
		.with_plutus_data(&unit_plutus_data())
		.next()?;
	let ma = MultiAsset::new().with_asset_amount(
		&scripts.illiquid_circulation_supply_auth_token_policy.empty_name_asset(),
		1u64,
	)?;

	amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()
}
