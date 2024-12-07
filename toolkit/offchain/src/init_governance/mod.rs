use crate::{
	await_tx::{AwaitTx, FixedDelayRetries},
	csl::{get_first_validator_budget, key_hash_address, NetworkTypeExt},
	OffchainError,
};
use anyhow::anyhow;
use cardano_serialization_lib::*;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosTx,
};
use sidechain_domain::{MainchainAddressHash, MainchainPrivateKey, McTxHash, UtxoId, UtxoIndex};

#[cfg(test)]
mod tests;

pub(crate) mod transaction;

pub trait InitGovernance {
	#[allow(async_fn_in_trait)]
	async fn init_governance(
		&self,
		governance_authority: MainchainAddressHash,
		payment_key: MainchainPrivateKey,
		genesis_utxo_id: UtxoId,
	) -> Result<OgmiosTx, OffchainError>;
}

impl<T> InitGovernance for T
where
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
{
	async fn init_governance(
		&self,
		governance_authority: MainchainAddressHash,
		payment_key: MainchainPrivateKey,
		genesis_utxo_id: UtxoId,
	) -> Result<OgmiosTx, OffchainError> {
		run_init_governance(
			governance_authority,
			payment_key,
			Some(genesis_utxo_id),
			self,
			FixedDelayRetries::two_minutes(),
		)
		.await
		.map_err(|e| OffchainError::InternalError(e.to_string()))
	}
}

pub async fn run_init_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	governance_authority: MainchainAddressHash,
	payment_key: MainchainPrivateKey,
	genesis_utxo_id: Option<UtxoId>,
	client: &T,
	await_tx: A,
) -> anyhow::Result<OgmiosTx> {
	let payment_key = PrivateKey::from_normal_bytes(&payment_key.0)
		.expect("MainchainPrivateKey is a valid PrivateKey");

	let network = client.shelley_genesis_configuration().await?.network;

	let own_address = key_hash_address(&payment_key.to_public().hash(), network.to_csl());
	log::info!("✉️ Submitter address: {}", own_address.to_bech32(None).unwrap());

	let own_utxos = client.query_utxos(&[own_address.to_bech32(None)?]).await?;
	log::info!("💱 {} UTXOs available", own_utxos.len());
	let protocol_parameters = client.query_protocol_parameters().await?;

	let genesis_utxo = match genesis_utxo_id {
		None => {
			log::info!("⚙️ No genesis UTXO provided, will select one automatically...");
			let utxo = own_utxos.first().ok_or(anyhow!("No UTXOs to choose from"))?.clone();
			log::info!("☑️ UTXO selected: {}", utxo);
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
		payment_key_utxos: own_utxos,
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

	log::info!("📨 Submitting transaction..");

	let all_costs = client.evaluate_transaction(&unsigned_transaction.to_bytes()).await?;
	let cost = get_first_validator_budget(all_costs)?;

	let unsigned_transaction = transaction::init_governance_transaction(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		governance_authority,
		&tx_context,
		genesis_utxo,
		cost,
	)?;
	let signed_transaction = tx_context.sign(&unsigned_transaction);

	let result = client.submit_transaction(&signed_transaction.to_bytes()).await?;
	let tx_id = result.transaction.id;
	log::info!("✅ Transaction submited. ID: {}", hex::encode(result.transaction.id));
	await_tx
		.await_tx_output(client, UtxoId { tx_hash: McTxHash(tx_id), index: UtxoIndex(0) })
		.await?;

	Ok(result.transaction)
}
