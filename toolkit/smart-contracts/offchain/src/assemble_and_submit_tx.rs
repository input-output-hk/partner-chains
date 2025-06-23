use crate::await_tx::AwaitTx;
use anyhow::anyhow;
use cardano_serialization_lib::{Transaction, Vkeywitness, Vkeywitnesses};
use ogmios_client::query_ledger_state::QueryUtxoByUtxoId;
use ogmios_client::{
	query_ledger_state::QueryLedgerState, query_network::QueryNetwork, transactions::Transactions,
};
use sidechain_domain::McTxHash;

/// Adds `witnesses` to `transaction` and submits it with `ogmios_client`.
pub async fn assemble_and_submit_tx<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	transaction: Transaction,
	witnesses: Vec<Vkeywitness>,
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let mut witness_set = transaction.witness_set();

	let mut vk = witness_set.vkeys().unwrap_or_else(Vkeywitnesses::new);

	for w in witnesses.iter() {
		vk.add(w);
	}
	witness_set.set_vkeys(&vk);

	let new_tx = Transaction::new(&transaction.body(), &witness_set, transaction.auxiliary_data());

	let res = ogmios_client.submit_transaction(&new_tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Submit assembled transaction request failed: {}, bytes: {}",
			e,
			hex::encode(new_tx.to_bytes())
		)
	})?;
	let tx_id = McTxHash(res.transaction.id);
	log::info!("Transaction submitted: {}", hex::encode(tx_id.0));

	await_tx.await_tx_output(ogmios_client, tx_id).await?;

	Ok(tx_id)
}
