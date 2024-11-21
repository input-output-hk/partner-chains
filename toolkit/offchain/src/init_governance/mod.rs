use crate::csl::{key_hash_address, NetworkTypeExt};
use anyhow::anyhow;
use cardano_serialization_lib::*;
use ogmios_client::{
	query_ledger_state::QueryLedgerState, query_network::QueryNetwork, transactions::Transactions,
	types::OgmiosTx,
};
use sidechain_domain::{MainchainAddressHash, MainchainPrivateKey, UtxoId};

#[cfg(test)]
mod tests;

pub(crate) mod transaction;

pub async fn run_init_governance(
	governance_authority: MainchainAddressHash,
	payment_key: MainchainPrivateKey,
	genesis_utxo_id: Option<UtxoId>,
	client: impl QueryLedgerState + Transactions + QueryNetwork,
) -> anyhow::Result<OgmiosTx> {
	let payment_key = PrivateKey::from_normal_bytes(&payment_key.0)
		.expect("MainchainPrivateKey is a valid PrivateKey");

	let network = client.shelley_genesis_configuration().await?.network;

	let own_address = key_hash_address(&payment_key.to_public().hash(), network.to_csl());
	println!("✉️ Submitter address: {}", own_address.to_bech32(None).unwrap());

	let own_utxos = client.query_utxos(&[own_address.to_bech32(None)?]).await?;
	println!("💱 {} UTXOs available", own_utxos.len());
	let protocol_parameters = client.query_protocol_parameters().await?;

	let genesis_utxo = match genesis_utxo_id {
		None => {
			println!("⚙️ No genesis UTXO provided, will select one automatically...");
			let utxo = own_utxos.first().ok_or(anyhow!("No UTXOs to choose from"))?.clone();
			println!("☑️ UTXO selected: {}", utxo);
			utxo
		},
		Some(utxo_id) => own_utxos
			.iter()
			.find(|utxo| utxo.transaction.id == utxo_id.tx_hash.0 && utxo.index == utxo_id.index.0)
			.ok_or(anyhow!("Could not find genesis UTXO: {utxo_id}"))?
			.clone(),
	};

	let tx_context = crate::csl::TransactionContext {
		payment_key,
		payment_utxos: own_utxos,
		network: network.to_csl(),
		protocol_parameters,
	};

	let unsigned_transaction = transaction::init_governance_transaction(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		governance_authority,
		&tx_context,
		genesis_utxo.clone(),
		ExUnits::new(&0u64.into(), &0u64.into()),
	)?;

	println!("📨 Submitting transaction:");
	println!("{}", unsigned_transaction.to_json()?);

	let all_costs = client.evaluate_transaction(&unsigned_transaction.to_bytes()).await?;
	let cost = crate::csl::convert_ex_units(&all_costs.first().unwrap().budget);

	let unsigned_transaction = transaction::init_governance_transaction(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		governance_authority,
		&tx_context,
		genesis_utxo,
		cost,
	)?;
	let signed_transaction = tx_context.sign(unsigned_transaction);

	let result = client.submit_transaction(&signed_transaction.to_bytes()).await?;

	println!("✅ Transaction submited. ID: {}", hex::encode(result.transaction.id));

	Ok(result.transaction)
}
