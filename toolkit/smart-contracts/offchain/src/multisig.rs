use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{get_builder_config, key_hash_address, Costs, TransactionBuilderExt, TransactionContext},
	governance::GovernanceData,
};
use cardano_serialization_lib::{
	PrivateKey, Transaction, TransactionBuilder, TransactionOutput, Value,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use serde::{Serialize, Serializer};
use sidechain_domain::{byte_string::ByteString, McTxHash, UtxoId, UtxoIndex};

/// Successfull smart contracts offchain results in either transaction submission or creating transaction that has to be signed by the governance authorities
#[derive(Clone, Debug, Serialize)]
pub enum MultiSigSmartContractResult {
	TransactionSubmitted(McTxHash),
	TransactionToSign(MultiSigTransactionData),
}

/// MultiSig transactions awaiting for signatures use temporary wallets where funds are stored until the transaction is signed and submitted.
/// This prevents payment utxo from being spend when the signatures for MultiSig are being collected.
#[derive(Clone, Debug, Serialize)]
pub struct MultiSigTransactionData {
	pub temporary_wallet: TemporaryWalletData,
	#[serde(serialize_with = "serialize_as_conway_tx")]
	pub tx_cbor: Vec<u8>,
}

/// To be used only for manual re-claim of the funds if transaction has not been submitted
#[derive(Clone, Debug, Serialize)]
pub struct TemporaryWalletData {
	pub address: String,
	pub private_key: ByteString,
	pub funded_by_tx: McTxHash,
}

pub(crate) struct TemporaryWallet {
	pub address: cardano_serialization_lib::Address,
	pub private_key: CardanoPaymentSigningKey,
	pub funded_by_tx: [u8; 32],
}

impl From<TemporaryWallet> for TemporaryWalletData {
	fn from(value: TemporaryWallet) -> Self {
		TemporaryWalletData {
			address: value.address.to_bech32(None).unwrap(),
			private_key: value.private_key.to_bytes().into(),
			funded_by_tx: McTxHash(value.funded_by_tx),
		}
	}
}

fn serialize_as_conway_tx<S>(tx_bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	let json = serde_json::json!({
		"type": "Tx ConwayEra",
		"description": "",
		"cborHex": hex::encode(tx_bytes)
	});
	json.serialize(serializer)
}

impl MultiSigSmartContractResult {
	pub fn tx_submitted(hash: [u8; 32]) -> Self {
		Self::TransactionSubmitted(McTxHash(hash))
	}
}

pub(crate) async fn create_temporary_wallet<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	ctx: &TransactionContext,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<TemporaryWallet> {
	let private_key = CardanoPaymentSigningKey(PrivateKey::generate_ed25519()?);
	log::info!(
		"Temporary wallet private key: {}. Store this key for eventual re-claim of tokens",
		private_key.0.to_hex()
	);
	let address = key_hash_address(&private_key.0.to_public().hash(), ctx.network);

	let mut funding_tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// TODO: ETCM-9627 - estimate tokens needed for the transaction
	funding_tx_builder
		.add_output(&TransactionOutput::new(&address, &Value::new(&5_000_000u32.into())))?;
	let funding_tx = funding_tx_builder.balance_update_and_build(ctx)?;
	let funding_tx_result = client.submit_transaction(&ctx.sign(&funding_tx).to_bytes()).await?;
	let funded_by_tx = funding_tx_result.transaction.id;
	await_tx.await_tx_output(client, UtxoId::new(funded_by_tx, 0)).await?;
	let address_str: String = address.to_bech32(None)?;
	log::info!(
		"Founded temporary wallet {} with 5 ADA in transaction: {}",
		&address_str,
		&hex::encode(funded_by_tx)
	);
	Ok(TemporaryWallet { address, private_key, funded_by_tx })
}

/// If the chain has real MultiSig governance it:
/// * creates a temporary wallet
/// * sends 5 ADA from the payment wallet (subject of change)
/// * creates a transaction that would be paid from the temporary wallet, signed by both wallets.
/// If the chain has single key governance it creates and submits transaction paid by and signed by the payment wallet.
pub(crate) async fn multisig_process<F, T, A>(
	governance_data: &GovernanceData,
	payment_ctx: &TransactionContext,
	make_tx: F,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult>
where
	F: Fn(Costs, &TransactionContext) -> anyhow::Result<Transaction>,
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
{
	if governance_data.policy.is_single_key_policy_for(&payment_ctx.payment_key_hash()) {
		let tx = Costs::calculate_costs(|c| make_tx(c, &payment_ctx), client).await?;
		let signed_tx = payment_ctx.sign(&tx).to_bytes();
		let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
			anyhow::anyhow!(
				"Submit governance update transaction request failed: {}, bytes: {}",
				e,
				hex::encode(signed_tx)
			)
		})?;
		let tx_id = McTxHash(res.transaction.id);
		log::info!("Update Governance transaction submitted: {}", hex::encode(tx_id.0));
		await_tx
			.await_tx_output(
				client,
				UtxoId { tx_hash: McTxHash(res.transaction.id), index: UtxoIndex(0) },
			)
			.await?;
		Ok(MultiSigSmartContractResult::TransactionSubmitted(tx_id))
	} else {
		let temporary_wallet = create_temporary_wallet(&payment_ctx, client, await_tx).await?;
		let temp_wallet_ctx = TransactionContext::for_payment_key_with_change_address(
			&temporary_wallet.private_key,
			&payment_ctx.change_address,
			client,
		)
		.await?;
		let tx = Costs::calculate_costs(|c| make_tx(c, &temp_wallet_ctx), client).await?;
		let signed_tx_by_caller = payment_ctx.sign(&tx);
		let signed_tx = temp_wallet_ctx.sign(&signed_tx_by_caller);
		Ok(MultiSigSmartContractResult::TransactionToSign(MultiSigTransactionData {
			temporary_wallet: temporary_wallet.into(),
			tx_cbor: signed_tx.to_bytes(),
		}))
	}
}
