use std::borrow::Borrow;

use crate::{await_tx::AwaitTx, csl::*, plutus_script::PlutusScript};
use anyhow::anyhow;
use cardano_serialization_lib::{
	Address, DataCost, LanguageKind, MinOutputAdaCalculator, ScriptRef, Transaction,
	TransactionBuilder, TransactionOutput, TransactionOutputBuilder,
};
use ogmios_client::{
	query_ledger_state::*, query_network::QueryNetwork, transactions::Transactions,
};
use sidechain_domain::{MainchainAddress, McTxHash, UtxoId};

pub async fn create_v_function_utxo<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	target_address: MainchainAddress,
	unix_start_time: u128,
	payment_key: [u8; 32],
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;

	let (tx, script_output_ix) = create_v_function_utxo_tx(&ctx, target_address, unix_start_time)?;
	let signed_tx = ctx.sign(&tx).to_bytes();

	let tx_id = client.submit_transaction(&signed_tx).await?.transaction.id;
	log::info!("Transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;
	log::info!(
		"Transaction confirmed. V-Function script reference in UTXO: {tx_id}{script_output_ix}",
	);
	Ok(Some(McTxHash(tx_id)))
}

fn create_v_function_utxo_tx(
	ctx: &TransactionContext,
	target_address: MainchainAddress,
	unix_start_time: u128,
) -> Result<(Transaction, usize), anyhow::Error> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let address = Address::from_bech32(&String::from_utf8(target_address.bytes())?.borrow())?;

	let v_function_script = PlutusScript::from_wrapped_cbor(
		raw_scripts::EXAMPLE_V_FUNCTION_POLICY,
		LanguageKind::PlutusV2,
	)?
	.apply_data(unix_start_time)?;

	tx_builder.add_output(&{
		let output_builder = TransactionOutputBuilder::new()
			.with_address(&address)
			.with_script_ref(&ScriptRef::new_plutus_script(&v_function_script.to_csl()))
			.next()?;
		let output = output_builder.with_coin(&0u64.into()).build()?;
		let min_ada = MinOutputAdaCalculator::new(
			&output,
			&DataCost::new_coins_per_byte(
				&ctx.protocol_parameters.min_utxo_deposit_coefficient.into(),
			),
		)
		.calculate_ada()?;
		output_builder.with_coin(&min_ada).build()?
	})?;

	let tx = tx_builder.balance_update_and_build(ctx)?;

	let ix = (tx.body().outputs().into_iter())
		.position(|utxo| utxo.has_script_ref())
		.ok_or_else(|| {
			anyhow!("BUG: created transaction does not have an output with a script reference")
		})?;

	Ok((tx, ix))
}
