use crate::{
	OffchainError,
	await_tx::{AwaitTx, FixedDelayRetries},
	cardano_keys::CardanoPaymentSigningKey,
	csl::Costs,
	csl::key_hash_address,
	governance::MultiSigParameters,
};
use anyhow::anyhow;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use sidechain_domain::{McTxHash, UtxoId};

#[cfg(test)]
mod tests;

pub(crate) mod transaction;

pub trait InitGovernance {
	/// Initializes goveranance mechanism with the authority being `governance_authority`,
	/// for the chain identified by `genesis_utxo_id`.
	#[allow(async_fn_in_trait)]
	async fn init_governance(
		&self,
		retries: FixedDelayRetries,
		governance_parameters: &MultiSigParameters,
		payment_key: &CardanoPaymentSigningKey,
		genesis_utxo_id: UtxoId,
	) -> Result<McTxHash, OffchainError>;
}

impl<T> InitGovernance for T
where
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
{
	async fn init_governance(
		&self,
		retries: FixedDelayRetries,
		governance_parameters: &MultiSigParameters,
		payment_key: &CardanoPaymentSigningKey,
		genesis_utxo_id: UtxoId,
	) -> Result<McTxHash, OffchainError> {
		run_init_governance(
			governance_parameters,
			payment_key,
			Some(genesis_utxo_id),
			self,
			retries,
		)
		.await
		.map(|result| result.tx_hash)
		.map_err(|e| OffchainError::InternalError(e.to_string()))
	}
}

#[derive(serde::Serialize)]
pub struct InitGovernanceResult {
	pub tx_hash: McTxHash,
	pub genesis_utxo: UtxoId,
}

pub async fn run_init_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	governance_parameters: &MultiSigParameters,
	payment_key: &CardanoPaymentSigningKey,
	genesis_utxo_id: Option<UtxoId>,
	client: &T,
	await_tx: A,
) -> anyhow::Result<InitGovernanceResult> {
	let ctx = crate::csl::TransactionContext::for_payment_key(payment_key, client).await?;

	let own_address = key_hash_address(&ctx.payment_key_hash(), ctx.network);
	log::info!("✉️ Submitter address: {}", own_address.to_bech32(None).unwrap());

	let own_utxos = ctx.payment_key_utxos.clone();
	log::info!("💱 {} UTXOs available", own_utxos.len());

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

	let tx = Costs::calculate_costs(
		|costs| {
			transaction::init_governance_transaction(
				governance_parameters,
				genesis_utxo.clone(),
				costs,
				&ctx,
			)
		},
		client,
	)
	.await?;

	let signed_transaction = ctx.sign(&tx);

	let result = client.submit_transaction(&signed_transaction.to_bytes()).await?;
	let tx_id = result.transaction.id;
	log::info!("✅ Transaction submitted. ID: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;
	Ok(InitGovernanceResult { tx_hash: McTxHash(tx_id), genesis_utxo: genesis_utxo.utxo_id() })
}
