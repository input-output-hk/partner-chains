use crate::{await_tx::AwaitTx, csl::*};
use ogmios_client::{
	query_ledger_state::*, query_network::QueryNetwork, transactions::Transactions,
	types::OgmiosUtxo,
};
use sidechain_domain::{McTxHash, UtxoId};

pub async fn release_reserve_funds<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Vec<McTxHash>> {
	todo!()
}

async fn get_utxo_with_tokens<T: QueryLedgerState>(
	ctx: &TransactionContext,
	client: &T,
) -> Result<Option<OgmiosUtxo>, anyhow::Error> {
	todo!()
}
