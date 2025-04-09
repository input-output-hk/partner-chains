use crate::csl::{
	empty_asset_name, get_builder_config, unit_plutus_data, CostStore, Costs,
	TransactionBuilderExt, TransactionExt,
};
use crate::governance::GovernanceData;
use crate::multisig::submit_or_create_tx_to_sign;
use crate::multisig::MultiSigSmartContractResult::TransactionSubmitted;
use crate::plutus_script::PlutusScript;
use crate::{
	await_tx::{AwaitTx, FixedDelayRetries},
	cardano_keys::CardanoPaymentSigningKey,
	csl::TransactionContext,
	multisig::MultiSigSmartContractResult,
};
use anyhow::anyhow;
use cardano_serialization_lib::{PlutusData, Transaction, TransactionBuilder};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::key_value::{key_value_to_plutus_data, KeyValueDatum};
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::{KeyValue, PolicyId, UtxoId};

pub trait KeyValueStore {
	#[allow(async_fn_in_trait)]
	async fn insert_key_value(
		&self,
		genesis_utxo: UtxoId,
		key: String,
		value: ByteString,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>>;
}

impl<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId> KeyValueStore for C {
	async fn insert_key_value(
		&self,
		genesis_utxo: UtxoId,
		key: String,
		value: ByteString,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
		insert_key_value(
			genesis_utxo,
			key,
			value,
			payment_signing_key,
			self,
			&FixedDelayRetries::two_minutes(),
		)
		.await
	}
}

pub async fn insert_key_value<
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
	let (validator, policy) =
		crate::scripts_data::generic_container_scripts(genesis_utxo, ctx.network)?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[validator_address]).await?;

	let tx_hash_opt = match get_current_key_value(validator_utxos, key.clone(), policy.policy_id())?
	{
		Some(current_value) => {
			log::info!("There is already a value stored for key '{}': {:?}", key, current_value);
			None
		},
		None => {
			log::info!("There is no value stored for key '{}'. Inserting new one.", key);
			Some(insert(&validator, &policy, key, value, ctx, genesis_utxo, ogmios_client).await?)
		},
	};
	if let Some(TransactionSubmitted(tx_hash)) = tx_hash_opt {
		await_tx.await_tx_output(ogmios_client, UtxoId::new(tx_hash.0, 0)).await?;
	}
	Ok(tx_hash_opt)
}

fn get_current_key_value(
	validator_utxos: Vec<OgmiosUtxo>,
	key: String,
	token: PolicyId,
) -> Result<Option<ByteString>, anyhow::Error> {
	for utxo in validator_utxos {
		if let None = utxo.value.native_tokens.get(&token.0) {
			continue;
		}
		if let Some(datum) = utxo.datum {
			let datum_plutus_data =
				PlutusData::from_bytes(datum.bytes).map_err(|e| {
					anyhow!("Internal error: could not decode datum of D-parameter validator script: {}", e)
				})?;
			let datum_key =
				KeyValueDatum::try_from(datum_plutus_data).map_err(|e| {
					anyhow!("Internal error: could not decode datum of D-parameter validator script: {}", e)
				})?;
			if datum_key.key == key {
				return Ok(Some(datum_key.value));
			}
		}
	}
	Ok(None)
}

async fn insert<C: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	key: String,
	value: ByteString,
	ctx: TransactionContext,
	genesis_utxo: UtxoId,
	client: &C,
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
		&FixedDelayRetries::two_minutes(),
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
		&key_value_to_plutus_data(&KeyValue::new(key, value)),
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

#[cfg(test)]
mod tests;
