//! D-parameter is stored on chain in an UTXO at the D-parameter validator address.
//! There should be at most one UTXO at the validator address and it should contain the D-parameter.
//! This UTXO should have 1 token of the D-parameter policy with an empty asset name.
//! The datum encodes D-parameter using VersionedGenericDatum envelope with the D-parameter being
//! `datum` field being `[num_permissioned_candidates, num_registered_candidates]`.

use crate::await_tx::{AwaitTx, FixedDelayRetries};
use crate::cardano_keys::CardanoPaymentSigningKey;
use crate::csl::{
	CostStore, Costs, InputsBuilderExt, NetworkTypeExt, TransactionBuilderExt, TransactionContext,
	TransactionExt, empty_asset_name, get_builder_config, unit_plutus_data,
};
use crate::governance::GovernanceData;
use crate::multisig::submit_or_create_tx_to_sign;
use crate::multisig::{
	MultiSigSmartContractResult, MultiSigSmartContractResult::TransactionSubmitted,
};
use crate::plutus_script::PlutusScript;
use anyhow::anyhow;
use cardano_serialization_lib::{PlutusData, Transaction, TransactionBuilder, TxInputsBuilder};
use ogmios_client::query_ledger_state::QueryUtxoByUtxoId;
use ogmios_client::{
	query_ledger_state::QueryLedgerState, query_network::QueryNetwork, transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::d_param::{DParamDatum, d_parameter_to_plutus_data};
use sidechain_domain::{DParameter, UtxoId};

#[cfg(test)]
mod tests;

/// Upserts D-param.
pub trait UpsertDParam {
	#[allow(async_fn_in_trait)]
	/// This function upserts D-param.
	/// Arguments:
	///  - `await_tx`: Configuration for the await logic of the transaction.
	///  - `genesis_utxo`: UTxO identifying the Partner Chain.
	///  - `d_parameter`: [DParameter] to be upserted.
	///  - `payment_signing_key`: Signing key of the party paying fees.
	async fn upsert_d_param(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		d_parameter: &DParameter,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>>;
}

impl<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId> UpsertDParam for C {
	async fn upsert_d_param(
		&self,
		await_tx: FixedDelayRetries,
		genesis_utxo: UtxoId,
		d_parameter: &DParameter,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
		upsert_d_param(genesis_utxo, d_parameter, payment_signing_key, self, &await_tx).await
	}
}

/// This function upserts D-param.
/// Arguments:
///  - `genesis_utxo`: UTxO identifying the Partner Chain.
///  - `d_parameter`: [DParameter] to be upserted.
///  - `payment_signing_key`: Signing key of the party paying fees.
///  - `await_tx`: [AwaitTx] strategy.
pub async fn upsert_d_param<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	d_parameter: &DParameter,
	payment_signing_key: &CardanoPaymentSigningKey,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, client).await?;
	let scripts = crate::scripts_data::d_parameter_scripts(genesis_utxo, ctx.network)?;
	let validator_utxos = client.query_utxos(&[scripts.validator_address.clone()]).await?;

	let tx_hash_opt = match get_current_d_parameter(validator_utxos)? {
		Some((_, current_d_param)) if current_d_param == *d_parameter => {
			log::info!("Current D-parameter value is equal to the one to be set.");
			None
		},
		Some((current_utxo, _)) => {
			log::info!("Current D-parameter is different to the one to be set. Updating.");
			Some(
				update_d_param(
					&scripts.validator,
					&scripts.policy,
					d_parameter,
					&current_utxo,
					ctx,
					genesis_utxo,
					client,
					await_tx,
				)
				.await?,
			)
		},
		None => {
			log::info!("There is no D-parameter set. Inserting new one.");
			Some(
				insert_d_param(
					&scripts.validator,
					&scripts.policy,
					d_parameter,
					ctx,
					genesis_utxo,
					client,
					await_tx,
				)
				.await?,
			)
		},
	};
	if let Some(TransactionSubmitted(tx_hash)) = tx_hash_opt {
		await_tx.await_tx_output(client, UtxoId::new(tx_hash.0, 0)).await?;
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
		let current_d_param: DParameter = DParamDatum::try_from(datum_plutus_data)
			.map_err(|e| {
				anyhow!(
					"Internal error: could not decode datum of D-parameter validator script: {}",
					e
				)
			})?
			.into();
		Ok(Some((utxo.clone(), current_d_param)))
	} else {
		Ok(None)
	}
}

async fn insert_d_param<
	C: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	ctx: TransactionContext,
	genesis_utxo: UtxoId,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult> {
	let gov_data = GovernanceData::get(genesis_utxo, client).await?;

	submit_or_create_tx_to_sign(
		&gov_data,
		ctx,
		|costs, ctx| mint_d_param_token_tx(validator, policy, d_parameter, &gov_data, costs, &ctx),
		"Insert D-parameter",
		client,
		await_tx,
	)
	.await
}

async fn update_d_param<
	C: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	current_utxo: &OgmiosUtxo,
	ctx: TransactionContext,
	genesis_utxo: UtxoId,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult> {
	let governance_data = GovernanceData::get(genesis_utxo, client).await?;

	submit_or_create_tx_to_sign(
		&governance_data,
		ctx,
		|costs, ctx| {
			update_d_param_tx(
				validator,
				policy,
				d_parameter,
				current_utxo,
				&governance_data,
				costs,
				&ctx,
			)
		},
		"Update D-parameter",
		client,
		await_tx,
	)
	.await
}

fn mint_d_param_token_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// The essence of transaction: mint D-Param token and set output with it, mint a governance token.
	tx_builder.add_mint_one_script_token(
		policy,
		&empty_asset_name(),
		&unit_plutus_data(),
		&costs.get_mint(&policy.clone()),
	)?;
	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

fn update_d_param_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	d_parameter: &DParameter,
	script_utxo: &OgmiosUtxo,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let mut inputs = TxInputsBuilder::new();
	inputs.add_script_utxo_input(
		script_utxo,
		validator,
		&unit_plutus_data(),
		&costs.get_one_spend(),
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
		&governance_data.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

/// Returns D-parameter.
pub trait GetDParam {
	#[allow(async_fn_in_trait)]
	/// Returns D-parameter.
	async fn get_d_param(&self, genesis_utxo: UtxoId) -> anyhow::Result<Option<DParameter>>;
}

impl<C: QueryLedgerState + QueryNetwork> GetDParam for C {
	async fn get_d_param(&self, genesis_utxo: UtxoId) -> anyhow::Result<Option<DParameter>> {
		get_d_param(genesis_utxo, self).await
	}
}

/// Returns D-parameter.
pub async fn get_d_param<C: QueryLedgerState + QueryNetwork>(
	genesis_utxo: UtxoId,
	client: &C,
) -> anyhow::Result<Option<DParameter>> {
	let network = client.shelley_genesis_configuration().await?.network.to_csl();
	let scripts = crate::scripts_data::d_parameter_scripts(genesis_utxo, network)?;
	let validator_utxos = client.query_utxos(&[scripts.validator_address.clone()]).await?;
	Ok(get_current_d_parameter(validator_utxos)?.map(|(_, d_parameter)| d_parameter))
}
