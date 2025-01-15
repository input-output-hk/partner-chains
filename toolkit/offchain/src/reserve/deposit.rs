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

use super::ReserveData;
use crate::{
	await_tx::AwaitTx,
	csl::{
		empty_asset_name, get_builder_config, get_validator_budgets, zero_ex_units, OgmiosUtxoExt,
		OgmiosValueExt, TransactionBuilderExt, TransactionContext,
	},
	init_governance::{get_governance_data, GovernanceData},
	reserve::get_reserve_data,
	scripts_data::ReserveScripts,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	AssetName, Assets, DataCost, ExUnits, JsError, Language, MinOutputAdaCalculator, MultiAsset,
	PlutusData, PlutusScriptSource, PlutusWitness, Redeemer, RedeemerTag, Transaction,
	TransactionBuilder, TransactionOutput, TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::{OgmiosEvaluateTransactionResponse, Transactions},
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::reserve::{ReserveDatum, ReserveRedeemer};
use sidechain_domain::{AssetId, McTxHash, UtxoId};

pub struct TokenAmount {
	pub token: AssetId,
	pub amount: u64,
}

/// Spends current UTXO at validator address and creates a new UTXO with increased token amount
pub async fn deposit_to_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	parameters: TokenAmount,
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	client: &T,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = get_governance_data(genesis_utxo, client).await?;
	let reserve = get_reserve_data(genesis_utxo, &ctx, client).await?;

	let utxo = get_utxo_with_tokens(&reserve.scripts, &parameters.token, &ctx, client).await?
		.ok_or_else(||anyhow!("There are no UTXOs in the Reserve Validator address that contain token Reserve Auth Policy Token. Has Reserve been created already?"))?;
	let current_amount = get_token_amount(&utxo, &parameters.token);
	let token_amount =
		TokenAmount { token: parameters.token, amount: current_amount + parameters.amount };

	let tx_to_evaluate = deposit_to_reserve_tx(
		&token_amount,
		&utxo,
		&reserve,
		&governance,
		zero_ex_units(),
		zero_ex_units(),
		&ctx,
	)?;
	let evaluate_response = client.evaluate_transaction(&tx_to_evaluate.to_bytes()).await?;

	let reserve_auth_ex_units = get_auth_policy_script_cost(evaluate_response.clone())?;
	let governance_ex_units = get_governance_script_cost(evaluate_response)?;

	let tx = deposit_to_reserve_tx(
		&token_amount,
		&utxo,
		&reserve,
		&governance,
		governance_ex_units,
		reserve_auth_ex_units,
		&ctx,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Deposit to Reserve transaction request failed: {}, tx bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!("Deposit to Reserve transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;
	Ok(McTxHash(tx_id))
}

async fn get_utxo_with_tokens<T: QueryLedgerState>(
	scripts: &ReserveScripts,
	token_id: &AssetId,
	ctx: &TransactionContext,
	client: &T,
) -> Result<Option<OgmiosUtxo>, anyhow::Error> {
	let validator_address = scripts.validator.address_bech32(ctx.network)?;
	let utxos = client.query_utxos(&[validator_address.clone()]).await?;
	Ok(utxos
		.into_iter()
		.find(|utxo| {
			utxo.value.native_tokens.contains_key(&scripts.auth_policy.script_hash())
				&& utxo.datum.clone().is_some_and(|datum| {
					decode_reserve_datum(datum).is_some_and(|reserve_datum| {
						reserve_datum.immutable_settings.token == *token_id
					})
				})
		})
		.clone())
}

fn decode_reserve_datum(datum: ogmios_client::types::Datum) -> Option<ReserveDatum> {
	PlutusData::from_bytes(datum.bytes)
		.ok()
		.and_then(|plutus_data| ReserveDatum::try_from(plutus_data).ok())
}

fn get_token_amount(utxo: &OgmiosUtxo, token: &AssetId) -> u64 {
	let AssetId { policy_id, asset_name } = token;
	utxo.value
		.native_tokens
		.get(&policy_id.0)
		.and_then(|assets| assets.iter().find(|asset| asset.name == asset_name.0.to_vec()))
		.map(|asset| asset.amount)
		.unwrap_or(0) // Token can be not found if the reserve was created with the initial deposit of 0 tokens
		.try_into()
		.expect("Token amount in an UTXO always fits u64")
}

fn deposit_to_reserve_tx(
	parameters: &TokenAmount,
	current_utxo: &OgmiosUtxo,
	reserve: &ReserveData,
	governance: &GovernanceData,
	governance_script_cost: ExUnits,
	reserve_auth_script_cost: ExUnits,
	ctx: &TransactionContext,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	tx_builder.add_output(&validator_output(parameters, current_utxo, &reserve.scripts, ctx)?)?;

	let inputs = input_with_script_reference(current_utxo, reserve, reserve_auth_script_cost)?;
	tx_builder.set_inputs(&inputs);

	let gov_tx_input = governance.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy_script,
		&gov_tx_input,
		&governance_script_cost,
	)?;

	tx_builder.add_script_reference_input(
		&reserve.auth_policy_version_utxo.to_csl_tx_input(),
		reserve.scripts.auth_policy.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&reserve.validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.validator.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&reserve.illiquid_circulation_supply_validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.illiquid_circulation_supply_validator.bytes.len(),
	);
	tx_builder.add_required_signer(&ctx.payment_key_hash());
	tx_builder.balance_update_and_build(ctx)
}

fn input_with_script_reference(
	consumed_utxo: &OgmiosUtxo,
	reserve: &ReserveData,
	cost: ExUnits,
) -> Result<TxInputsBuilder, JsError> {
	let mut inputs = TxInputsBuilder::new();
	let input = consumed_utxo.to_csl_tx_input();
	let amount = consumed_utxo.value.to_csl()?;
	let script = &reserve.scripts.auth_policy;
	let redeemer_data = ReserveRedeemer::DepositToReserve { governance_version: 1 }.into();
	let witness = PlutusWitness::new_with_ref_without_datum(
		&PlutusScriptSource::new_ref_input(
			&script.csl_script_hash(),
			&input,
			&Language::new_plutus_v2(),
			script.bytes.len(),
		),
		&Redeemer::new(&RedeemerTag::new_spend(), &0u32.into(), &redeemer_data, &cost),
	);
	inputs.add_plutus_script_input(&witness, &input, &amount);
	Ok(inputs)
}

// governance token is the only minted token
fn get_governance_script_cost(
	response: Vec<OgmiosEvaluateTransactionResponse>,
) -> Result<ExUnits, anyhow::Error> {
	Ok(get_validator_budgets(response)
		.mint_ex_units
		.first()
		.ok_or_else(|| anyhow!("Mint cost is missing in evaluate response"))?
		.clone())
}

// Auth policy token is the only spent token is the transaction
fn get_auth_policy_script_cost(
	response: Vec<OgmiosEvaluateTransactionResponse>,
) -> Result<ExUnits, anyhow::Error> {
	Ok(get_validator_budgets(response)
		.spend_ex_units
		.first()
		.ok_or_else(|| anyhow!("Spend cost is missing in evaluate response"))?
		.clone())
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
			&PlutusData::from_bytes(
				current_utxo
					.datum
					.clone()
					.expect("Current UTXO datum was parsed hence it exists")
					.bytes,
			)
			.unwrap(),
		)
		.next()?;
	let mut ma = MultiAsset::new();
	let mut assets = Assets::new();
	assets.insert(&empty_asset_name(), &1u64.into());
	ma.insert(&scripts.auth_policy.csl_script_hash(), &assets);
	let AssetId { policy_id, asset_name } = token_amount.token.clone();
	let mut assets = Assets::new();
	assets.insert(
		&AssetName::new(asset_name.0.to_vec()).expect("AssetName has a valid length"),
		&token_amount.amount.into(),
	);
	ma.insert(&policy_id.0.into(), &assets);
	let output = amount_builder.with_coin_and_asset(&0u64.into(), &ma).build()?;

	let ada = MinOutputAdaCalculator::new(
		&output,
		&DataCost::new_coins_per_byte(&ctx.protocol_parameters.min_utxo_deposit_coefficient.into()),
	)
	.calculate_ada()?;

	amount_builder.with_coin_and_asset(&ada, &ma).build()
}
