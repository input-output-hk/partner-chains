//! D-parameter is stored on chain in an UTXO at the D-parameter validator address.
//! There should be at most one UTXO at the validator address and it should contain the D-parameter.
//! This UTXO should have 1 token of the D-parameter policy with an empty asset name.
//! The datum encodes D-parameter using VersionedGenericDatum envelope with the D-parameter being
//! `datum` field being `[num_permissioned_candidates, num_registered_candidates]`.

use crate::await_tx::{AwaitTx, FixedDelayRetries};
use crate::csl::{
	get_builder_config, get_validator_budgets, zero_ex_units, InputsBuilderExt, ScriptExUnits,
	TransactionBuilderExt, TransactionContext,
};
use crate::init_governance::{self, GovernanceData};
use crate::plutus_script::PlutusScript;
use anyhow::anyhow;
use cardano_serialization_lib::{
	BigNum, ExUnits, JsError, PlutusData, ScriptHash, Transaction, TransactionBuilder,
	TxInputsBuilder,
};
use ogmios_client::query_ledger_state::QueryUtxoByUtxoId;
use ogmios_client::{
	query_ledger_state::QueryLedgerState, query_network::QueryNetwork, transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::d_param::{d_parameter_to_plutus_data, DParamDatum};
use sidechain_domain::{DParameter, McTxHash, UtxoId};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

pub trait UpsertDParam {
	#[allow(async_fn_in_trait)]
	async fn upsert_d_param(
		&self,
		genesis_utxo: UtxoId,
		d_parameter: &DParameter,
		payment_signing_key: [u8; 32],
	) -> anyhow::Result<Option<McTxHash>>;
}

impl<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId> UpsertDParam for C {
	async fn upsert_d_param(
		&self,
		genesis_utxo: UtxoId,
		d_parameter: &DParameter,
		payment_signing_key: [u8; 32],
	) -> anyhow::Result<Option<McTxHash>> {
		upsert_d_param(
			genesis_utxo,
			d_parameter,
			payment_signing_key,
			self,
			&FixedDelayRetries::two_minutes(),
		)
		.await
	}
}

pub async fn upsert_d_param<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	d_parameter: &DParameter,
	payment_signing_key: [u8; 32],
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let (validator, policy) = crate::scripts_data::d_parameter_scripts(genesis_utxo, ctx.network)?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[validator_address]).await?;

	let tx_hash_opt = match get_current_d_parameter(validator_utxos)? {
		Some((_, current_d_param)) if current_d_param == *d_parameter => {
			log::info!("Current D-parameter value is equal to the one to be set.");
			None
		},
		Some((current_utxo, _)) => {
			log::info!("Current D-parameter is different to the one to be set. Updating.");
			Some(
				update_d_param(
					&validator,
					&policy,
					d_parameter,
					&current_utxo,
					ctx,
					genesis_utxo,
					ogmios_client,
				)
				.await?,
			)
		},
		None => {
			log::info!("There is no D-parameter set. Inserting new one.");
			Some(
				insert_d_param(&validator, &policy, d_parameter, ctx, genesis_utxo, ogmios_client)
					.await?,
			)
		},
	};
	if let Some(tx_hash) = tx_hash_opt {
		await_tx.await_tx_output(ogmios_client, UtxoId::new(tx_hash.0, 0)).await?;
	}
	Ok(tx_hash_opt)
}

fn get_current_d_parameter(
	validator_utxos: Vec<OgmiosUtxo>,
) -> Result<Option<(OgmiosUtxo, DParameter)>, anyhow::Error> {
	if let Some(utxo) = validator_utxos.first() {
		let datum = utxo.datum.clone().ok_or_else(|| {
			anyhow!("Invalid state: an UTXO at the validator script address does not have a datum")
		})?;
		let datum_plutus_data = PlutusData::from_bytes(datum.bytes).map_err(|e| {
			anyhow!("Internal error: could not decode datum of D-parameter validator script: {}", e)
		})?;
		let current_d_param: DParameter =
			DParamDatum::try_from(datum_plutus_data)
				.map_err(|e| {
					anyhow!("Internal error: could not decode datum of D-parameter validator script: {}", e)
				})?
				.into();
		Ok(Some((utxo.clone(), current_d_param)))
	} else {
		Ok(None)
	}
}

async fn insert_d_param<C: QueryLedgerState + Transactions + QueryNetwork>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	ctx: TransactionContext,
	genesis_utxo: UtxoId,
	client: &C,
) -> anyhow::Result<McTxHash> {
	let gov_data = init_governance::get_governance_data(genesis_utxo, client).await?;

	let tx = mint_d_param_token_tx(
		validator,
		policy,
		d_parameter,
		&gov_data,
		&ctx,
		&zero_ex_units(),
		&zero_ex_units(),
	)?;

	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate insert D-parameter transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;

	let mint_keys = tx.body().mint().expect("insert D parameter transaction has two mints").keys();
	let script_to_index: HashMap<ScriptHash, usize> =
		vec![(mint_keys.get(0), 0), (mint_keys.get(1), 1)].into_iter().collect();
	let mint_ex_units = get_validator_budgets(evaluate_response).mint_ex_units;
	let policy_idx = *script_to_index.get(&policy.csl_script_hash()).unwrap();
	let gov_policy_idx = *script_to_index.get(&gov_data.policy_script.csl_script_hash()).unwrap();
	let policy_ex_units = mint_ex_units
		.get(policy_idx)
		.expect("Evaluate transaction response should have entry for d_param policy");
	let gov_policy_ex_units = mint_ex_units
		.get(gov_policy_idx)
		.expect("Evaluate transaction response should have entry for governance policy");

	let tx = mint_d_param_token_tx(
		validator,
		policy,
		d_parameter,
		&gov_data,
		&ctx,
		policy_ex_units,
		gov_policy_ex_units,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit insert D-parameter transaction request failed: {}, bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = McTxHash(res.transaction.id);
	log::info!("Transaction submitted: {}", hex::encode(tx_id.0));
	Ok(tx_id)
}

async fn update_d_param<C: QueryLedgerState + Transactions + QueryNetwork>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	current_utxo: &OgmiosUtxo,
	ctx: TransactionContext,
	genesis_utxo: UtxoId,
	client: &C,
) -> anyhow::Result<McTxHash> {
	let zero_ex_units = ScriptExUnits {
		mint_ex_units: vec![zero_ex_units()],
		spend_ex_units: vec![zero_ex_units()],
	};

	let governance_data = init_governance::get_governance_data(genesis_utxo, client).await?;

	let tx = update_d_param_tx(
		validator,
		policy,
		d_parameter,
		current_utxo,
		&governance_data,
		&ctx,
		zero_ex_units,
	)?;
	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate update D-parameter transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let spend_ex_units = get_validator_budgets(evaluate_response);

	let tx = update_d_param_tx(
		validator,
		policy,
		d_parameter,
		current_utxo,
		&governance_data,
		&ctx,
		spend_ex_units,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit D-parameter update transaction request failed: {}, bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = McTxHash(res.transaction.id);
	log::info!("Update D-parameter transaction submitted: {}", hex::encode(tx_id.0));
	Ok(tx_id)
}

fn mint_d_param_token_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	governance_data: &GovernanceData,
	ctx: &TransactionContext,
	d_param_policy_ex_units: &ExUnits,
	gov_policy_ex_units: &ExUnits,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// The essence of transaction: mint D-Param token and set output with it, mint a governance token.
	tx_builder.add_mint_one_script_token(
		policy,
		&d_param_redeemer_data(),
		d_param_policy_ex_units,
	)?;
	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy_script,
		&gov_tx_input,
		gov_policy_ex_units,
	)?;

	tx_builder.balance_update_and_build(ctx)
}

fn update_d_param_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	script_utxo: &OgmiosUtxo,
	governance_data: &GovernanceData,
	ctx: &TransactionContext,
	ex_units: ScriptExUnits,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let mut inputs = TxInputsBuilder::new();
	inputs.add_script_utxo_input(
		script_utxo,
		validator,
		&d_param_redeemer_data(),
		ex_units
			.spend_ex_units
			.first()
			.ok_or_else(|| JsError::from_str("Spend ex units not found"))?,
	)?;
	tx_builder.set_inputs(&inputs);

	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy_script,
		&gov_tx_input,
		ex_units
			.mint_ex_units
			.first()
			.ok_or_else(|| JsError::from_str("Mint ex units not found"))?,
	)?;

	tx_builder.balance_update_and_build(ctx)
}

/// D-param policy accepts any redeemer data.
fn d_param_redeemer_data() -> PlutusData {
	PlutusData::new_empty_constr_plutus_data(&BigNum::zero())
}
