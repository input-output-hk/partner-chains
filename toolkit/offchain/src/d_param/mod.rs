//! D-parameter is stored on chain in an UTXO at the D-parameter validator address.
//! There should be at most one UTXO at the validator address and it should contain the D-parameter.
//! This UTXO should have 1 token of the D-parameter policy with an empty asset name.
//! The datum encodes D-parameter using VersionedGenericDatum envelope with the D-parameter being
//! `datum` field being `[num_permissioned_candidates, num_registered_candidates]`.

use crate::csl::{
	get_builder_config, get_validator_budgets, InputsBuilderExt, ScriptExUnits,
	TransactionBuilderExt, TransactionContext,
};
use crate::plutus_script::PlutusScript;
use anyhow::anyhow;
use cardano_serialization_lib::{
	ExUnits, JsError, PlutusData, Transaction, TransactionBuilder, TxInputsBuilder,
};
use cardano_serialization_lib::{LanguageKind, TransactionHash, TransactionInput};
use ogmios_client::{
	query_ledger_state::QueryLedgerState, query_network::QueryNetwork, transactions::Transactions,
	types::OgmiosScript::Plutus, types::OgmiosUtxo,
};
use partner_chains_plutus_data::d_param::{d_parameter_to_plutus_data, DParamDatum};
use sidechain_domain::{DParameter, McTxHash, UtxoId};

#[cfg(test)]
mod tests;

pub async fn upsert_d_param<C: QueryLedgerState + QueryNetwork + Transactions>(
	genesis_utxo: UtxoId,
	d_parameter: &DParameter,
	payment_signing_key: [u8; 32],
	ogmios_client: &C,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let (validator, policy) = crate::scripts_data::d_parameter_scripts(genesis_utxo, ctx.network)?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[validator_address]).await?;

	match get_current_d_parameter(validator_utxos)? {
		Some((_, current_d_param)) if current_d_param == *d_parameter => {
			log::info!("Current D-parameter value is equal to the one to be set.");
			Ok(None)
		},
		Some((current_utxo, _)) => {
			log::info!("Current D-parameter is different to the one to be set. Updating.");
			Ok(Some(
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
			))
		},
		None => {
			log::info!("There is no D-parameter set. Inserting new one.");
			Ok(Some(
				insert_d_param(&validator, &policy, d_parameter, ctx, genesis_utxo, ogmios_client)
					.await?,
			))
		},
	}
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
) -> anyhow::Result<McTxHash>
where
	C: Transactions,
{
	let zero_ex_units = ScriptExUnits {
		mint_ex_units: vec![
			ExUnits::new(&0u64.into(), &0u64.into()),
			ExUnits::new(&0u64.into(), &0u64.into()),
		],
		spend_ex_units: vec![],
	};

	let gov_utxo = crate::init_governance::get_governance_utxo(genesis_utxo, client)
		.await
		.map_err(|e| JsError::from_str(e.to_string().as_str()))?;

	let tx = mint_d_param_token_tx(
		validator,
		policy,
		d_parameter,
		&ctx,
		zero_ex_units,
		gov_utxo.clone(),
	)?;
	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate insert D-parameter transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let mint_witness_ex_units = get_validator_budgets(evaluate_response)?;
	let tx = mint_d_param_token_tx(
		validator,
		policy,
		d_parameter,
		&ctx,
		mint_witness_ex_units,
		gov_utxo,
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
) -> anyhow::Result<McTxHash>
where
	C: Transactions,
{
	let zero_ex_units = ScriptExUnits {
		mint_ex_units: vec![ExUnits::new(&0u64.into(), &0u64.into())],
		spend_ex_units: vec![ExUnits::new(&0u64.into(), &0u64.into())],
	};

	let gov_utxo = crate::init_governance::get_governance_utxo(genesis_utxo, client)
		.await
		.map_err(|e| JsError::from_str(e.to_string().as_str()))?;

	let tx = update_d_param_tx(
		validator,
		policy,
		d_parameter,
		current_utxo,
		&ctx,
		zero_ex_units,
		gov_utxo.clone(),
	)?;
	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate update D-parameter transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let spend_ex_units = get_validator_budgets(evaluate_response)?;

	let tx = update_d_param_tx(
		validator,
		policy,
		d_parameter,
		current_utxo,
		&ctx,
		spend_ex_units,
		gov_utxo.clone(),
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
	ctx: &TransactionContext,
	mut ex_units: ScriptExUnits,
	gov_utxo: OgmiosUtxo,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// The essence of transaction: mint token and set output with it
	tx_builder.add_mint_one_script_token(
		policy,
		ex_units
			.mint_ex_units
			.pop()
			.unwrap_or_else(|| panic!("Mint ex units not found")),
	)?;
	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	let gov_policy = match gov_utxo.script {
		Some(Plutus(ps)) => PlutusScript::from_cbor(&ps.cbor, LanguageKind::PlutusV2),
		_ => return Err(JsError::from_str("Governance UTXO script is not PlutusScript")),
	};

	let gov_tx_input = TransactionInput::new(
		&TransactionHash::from_bytes(gov_utxo.transaction.id.into())?,
		gov_utxo.index.into(),
	);
	tx_builder.add_mint_one_script_token_using_reference_script(
		&gov_policy,
		&gov_tx_input,
		ex_units
			.mint_ex_units
			.pop()
			.unwrap_or_else(|| panic!("Mint ex units not found")),
	)?;

	let tx_hash = TransactionHash::from_bytes(gov_utxo.transaction.id.into())?;
	let gov_tx_input = TransactionInput::new(&tx_hash, gov_utxo.index.into());
	tx_builder.add_script_reference_input(&gov_tx_input, gov_policy.bytes.len());
	tx_builder.add_required_signer(&ctx.payment_key_hash());
	tx_builder.balance_update_and_build(ctx)
}

fn update_d_param_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	script_utxo: &OgmiosUtxo,
	ctx: &TransactionContext,
	mut ex_units: ScriptExUnits,
	gov_utxo: OgmiosUtxo,
) -> Result<Transaction, JsError> {
	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	let mut inputs = TxInputsBuilder::new();
	inputs.add_script_utxo_input(
		script_utxo,
		validator,
		ex_units
			.spend_ex_units
			.pop()
			.unwrap_or_else(|| panic!("Spend ex units not found")),
	)?;
	tx_builder.set_inputs(&inputs);

	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	let gov_policy = match gov_utxo.script {
		Some(Plutus(ps)) => PlutusScript::from_cbor(&ps.cbor, LanguageKind::PlutusV2),
		_ => return Err(JsError::from_str("Governance UTXO script is not PlutusScript")),
	};

	let gov_tx_input = TransactionInput::new(
		&TransactionHash::from_bytes(gov_utxo.transaction.id.into())?,
		gov_utxo.index.into(),
	);
	tx_builder.add_mint_one_script_token_using_reference_script(
		&gov_policy,
		&gov_tx_input,
		ex_units
			.mint_ex_units
			.pop()
			.unwrap_or_else(|| panic!("Mint ex units not found")),
	)?;

	let tx_hash = TransactionHash::from_bytes(gov_utxo.transaction.id.into())?;
	let gov_tx_input = TransactionInput::new(&tx_hash, gov_utxo.index.into());
	tx_builder.add_script_reference_input(&gov_tx_input, gov_policy.bytes.len());
	tx_builder.add_required_signer(&ctx.payment_key_hash());
	tx_builder.balance_update_and_build(ctx)
}
