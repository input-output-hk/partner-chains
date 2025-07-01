use crate::csl::{
	CostStore, Costs, InputsBuilderExt, NetworkTypeExt, TransactionBuilderExt, TransactionExt,
	empty_asset_name, get_builder_config, unit_plutus_data,
};
use crate::governance::GovernanceData;
use crate::multisig::MultiSigSmartContractResult::TransactionSubmitted;
use crate::multisig::submit_or_create_tx_to_sign;
use crate::plutus_script::PlutusScript;
use crate::{
	await_tx::AwaitTx, cardano_keys::CardanoPaymentSigningKey, csl::TransactionContext,
	multisig::MultiSigSmartContractResult,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	Int, PlutusData, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::governed_map::{
	GovernedMapDatum, governed_map_datum_to_plutus_data,
};
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::{PolicyId, UtxoId};
use std::ops::Neg;

#[cfg(test)]
mod tests;

/// Inserts an entry into the governed map.
/// If the `key` is already set to the provided `value` a transaction is not submitted and the operation succeeds.
/// Else if the `key` is already set, the operation fails.
pub async fn run_insert<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	key: String,
	value: ByteString,
	payment_signing_key: &CardanoPaymentSigningKey,
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let scripts = crate::scripts_data::governed_map_scripts(genesis_utxo, ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[scripts.validator_address.clone()]).await?;

	let tx_hash_opt = match get_current_value(validator_utxos, key.clone(), scripts.policy_id()) {
		Some(current_value) if current_value != value => {
			return Err(anyhow!("There is already a value stored for key '{key}'."));
		},
		Some(_current_value) => {
			log::info!("Value for key '{key}' is already set to the same value. Skipping insert.");
			None
		},
		None => {
			log::info!("There is no value stored for key '{key}'. Inserting new one.");
			Some(
				insert(
					&scripts.validator,
					&scripts.policy,
					key,
					value,
					ctx,
					genesis_utxo,
					ogmios_client,
					await_tx,
				)
				.await?,
			)
		},
	};
	if let Some(TransactionSubmitted(tx_hash)) = tx_hash_opt {
		await_tx.await_tx_output(ogmios_client, UtxoId::new(tx_hash.0, 0)).await?;
	}
	Ok(tx_hash_opt)
}

async fn insert<
	C: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	key: String,
	value: ByteString,
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
			insert_key_value_tx(
				validator,
				policy,
				key.clone(),
				value.clone(),
				&governance_data,
				costs,
				&ctx,
			)
		},
		"Insert Key-Value pair",
		client,
		await_tx,
	)
	.await
}

fn insert_key_value_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	key: String,
	value: ByteString,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	tx_builder.add_mint_one_script_token(
		policy,
		&empty_asset_name(),
		&unit_plutus_data(),
		&costs.get_mint(&policy.clone()),
	)?;
	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&governed_map_datum_to_plutus_data(&GovernedMapDatum::new(key, value)),
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

/// Updates an entry in the governed map.
/// If `expected_current_value` is provided, the current `value` for the `key` must match it, otherwise the operation fails.
/// If the `key` is not set, the operation fails.
/// If the `key` is already set to the provided `value` a transaction is not submitted and the operation succeeds.
pub async fn run_update<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	key: String,
	value: ByteString,
	expected_current_value: Option<ByteString>,
	payment_signing_key: &CardanoPaymentSigningKey,
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let scripts = crate::scripts_data::governed_map_scripts(genesis_utxo, ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[scripts.validator_address.clone()]).await?;
	let utxos_for_key =
		get_utxos_for_key(validator_utxos.clone(), key.clone(), scripts.policy_id());

	let Some(actual_current_value) =
		get_current_value(validator_utxos.clone(), key.clone(), scripts.policy_id())
	else {
		return Err(anyhow!("Cannot update nonexistent key :'{key}'."));
	};

	if matches!(expected_current_value,
		Some(ref expected_current_value) if *expected_current_value != actual_current_value)
	{
		return Err(anyhow!("Value for key '{key}' is set to a different value than expected."));
	}

	let tx_hash_opt = {
		if actual_current_value != value {
			Some(
				update(
					&scripts.validator,
					&scripts.policy,
					key,
					value,
					&utxos_for_key,
					ctx,
					genesis_utxo,
					ogmios_client,
					await_tx,
				)
				.await?,
			)
		} else {
			log::info!("Value for key '{key}' is already set to the same value. Skipping update.");
			None
		}
	};

	if let Some(TransactionSubmitted(tx_hash)) = tx_hash_opt {
		await_tx.await_tx_output(ogmios_client, UtxoId::new(tx_hash.0, 0)).await?;
	}
	Ok(tx_hash_opt)
}

async fn update<
	C: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	key: String,
	value: ByteString,
	utxos_for_key: &[OgmiosUtxo],
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
			update_key_value_tx(
				validator,
				policy,
				key.clone(),
				value.clone(),
				utxos_for_key,
				&governance_data,
				costs,
				&ctx,
			)
		},
		"Update Key-Value pair",
		client,
		await_tx,
	)
	.await
}

fn update_key_value_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	key: String,
	value: ByteString,
	utxos_for_key: &[OgmiosUtxo],
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	let spend_indicies = costs.get_spend_indices();

	let mut inputs = TxInputsBuilder::new();
	for (ix, utxo) in utxos_for_key.iter().enumerate() {
		inputs.add_script_utxo_input(
			utxo,
			validator,
			&PlutusData::new_bytes(vec![]),
			&costs.get_spend(*spend_indicies.get(ix).unwrap_or(&0)),
		)?;
	}
	tx_builder.set_inputs(&inputs);

	if utxos_for_key.len() > 1 {
		let burn_amount = (utxos_for_key.len() as i32 - 1).neg();
		tx_builder.add_mint_script_tokens(
			policy,
			&empty_asset_name(),
			&unit_plutus_data(),
			&costs.get_mint(&policy.clone()),
			&Int::new_i32(burn_amount),
		)?;
	}

	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&governed_map_datum_to_plutus_data(&GovernedMapDatum::new(key, value)),
		ctx,
	)?;

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

/// Removes an entry from the governed map.
/// If the `key` doesn't exist in the map a transaction is not submitted and the operation succeeds.
pub async fn run_remove<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	key: String,
	payment_signing_key: &CardanoPaymentSigningKey,
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let scripts = crate::scripts_data::governed_map_scripts(genesis_utxo, ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[scripts.validator_address.clone()]).await?;

	let utxos_for_key = get_utxos_for_key(validator_utxos, key.clone(), scripts.policy_id());

	let tx_hash_opt = match utxos_for_key.len() {
		0 => {
			log::info!("There is no value stored for key '{key}'. Skipping remove.");
			None
		},
		_ => Some(
			remove(
				&scripts.validator,
				&scripts.policy,
				&utxos_for_key,
				ctx,
				genesis_utxo,
				ogmios_client,
				await_tx,
			)
			.await?,
		),
	};
	if let Some(TransactionSubmitted(tx_hash)) = tx_hash_opt {
		await_tx.await_tx_output(ogmios_client, UtxoId::new(tx_hash.0, 0)).await?;
	}
	Ok(tx_hash_opt)
}

async fn remove<
	C: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	utxos_for_key: &Vec<OgmiosUtxo>,
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
			remove_key_value_tx(validator, policy, utxos_for_key, &governance_data, costs, &ctx)
		},
		"Remove Key-Value pair",
		client,
		await_tx,
	)
	.await
}

fn remove_key_value_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	utxos_for_key: &Vec<OgmiosUtxo>,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	let spend_indices = costs.get_spend_indices();

	let mut inputs = TxInputsBuilder::new();
	for (ix, utxo) in utxos_for_key.iter().enumerate() {
		inputs.add_script_utxo_input(
			utxo,
			validator,
			&unit_plutus_data(),
			&costs.get_spend(*spend_indices.get(ix).unwrap_or(&0)),
		)?;
	}
	tx_builder.set_inputs(&inputs);

	let burn_amount = (utxos_for_key.len() as i32).neg();
	tx_builder.add_mint_script_tokens(
		policy,
		&empty_asset_name(),
		&unit_plutus_data(),
		&costs.get_mint(&policy.clone()),
		&Int::new_i32(burn_amount),
	)?;

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

/// Queries all entries stored in the governed map.
pub async fn run_list<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId>(
	genesis_utxo: UtxoId,
	ogmios_client: &C,
) -> anyhow::Result<impl Iterator<Item = GovernedMapDatum>> {
	let network = ogmios_client.shelley_genesis_configuration().await?.network.to_csl();
	let scripts = crate::scripts_data::governed_map_scripts(genesis_utxo, network)?;
	let validator_utxos = ogmios_client.query_utxos(&[scripts.validator_address.clone()]).await?;
	Ok(ogmios_utxos_to_governed_map_utxos(validator_utxos.into_iter(), scripts.policy_id())
		.map(|(_, datum)| datum))
}

/// Queries the provided `key` from the governed map.
pub async fn run_get<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId>(
	genesis_utxo: UtxoId,
	key: String,
	ogmios_client: &C,
) -> anyhow::Result<Option<ByteString>> {
	let network = ogmios_client.shelley_genesis_configuration().await?.network.to_csl();
	let scripts = crate::scripts_data::governed_map_scripts(genesis_utxo, network)?;
	let validator_utxos = ogmios_client.query_utxos(&[scripts.validator_address.clone()]).await?;
	Ok(get_current_value(validator_utxos, key, scripts.policy_id()))
}

fn ogmios_utxos_to_governed_map_utxos(
	utxos: impl Iterator<Item = OgmiosUtxo>,
	token: PolicyId,
) -> impl Iterator<Item = (OgmiosUtxo, GovernedMapDatum)> {
	utxos.flat_map(move |utxo| {
		let _ = utxo.clone().value.native_tokens.get(&token.0)?;
		let datum = utxo.clone().datum?;
		let datum_plutus_data = PlutusData::from_bytes(datum.bytes).ok()?;

		GovernedMapDatum::try_from(datum_plutus_data).ok().and_then(|d| Some((utxo, d)))
	})
}

fn get_current_value(
	validator_utxos: Vec<OgmiosUtxo>,
	key: String,
	token: PolicyId,
) -> Option<ByteString> {
	ogmios_utxos_to_governed_map_utxos(validator_utxos.into_iter(), token)
		.find(|(_, datum)| datum.key == key)
		.map(|(_, datum)| datum.value)
}

fn get_utxos_for_key(
	validator_utxos: Vec<OgmiosUtxo>,
	key: String,
	token: PolicyId,
) -> Vec<OgmiosUtxo> {
	ogmios_utxos_to_governed_map_utxos(validator_utxos.into_iter(), token)
		.filter(|(_, datum)| datum.key == key)
		.map(|(utxo, _)| utxo)
		.collect()
}

/// Necessary to test rare case, where two inserts for the same key are executed
#[allow(dead_code)]
pub async fn run_insert_with_force<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	key: String,
	value: ByteString,
	payment_signing_key: &CardanoPaymentSigningKey,
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let scripts = crate::scripts_data::governed_map_scripts(genesis_utxo, ctx.network)?;

	let tx_hash_opt = Some(
		insert(
			&scripts.validator,
			&scripts.policy,
			key,
			value,
			ctx,
			genesis_utxo,
			ogmios_client,
			await_tx,
		)
		.await?,
	);

	if let Some(TransactionSubmitted(tx_hash)) = tx_hash_opt {
		await_tx.await_tx_output(ogmios_client, UtxoId::new(tx_hash.0, 0)).await?;
	}
	Ok(tx_hash_opt)
}
