//! D-parameter is stored on chain in an UTXO at the D-parameter validator address.
//! There should be at most one UTXO at the validator address and it should contain the D-parameter.
//! This UTXO should have 1 token of the D-parameter policy with an empty asset name.
//! The datum encodes D-parameter using VersionedGenericDatum envelope with the D-parameter being
//! `datum` field being `[num_permissioned_candidates, num_registered_candidates]`.


pub async fn init_reserve_management<T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,>(
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	client: &T,
)->Result<Option<McTxHash>> {
	Ok(None)
}

fn init_reserve_management_tx(
	_genesis_utxo: OgmiosUtxo,
	_tx_context: &TransactionContext,
	_ex_units: ExUnits,
) -> anyhow::Result<Transaction> {

	anyhow::Error::msg("Not implemented")
}
