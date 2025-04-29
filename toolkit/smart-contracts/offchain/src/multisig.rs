use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		Costs, OgmiosUtxoExt, TransactionBuilderExt, TransactionContext, get_builder_config,
		key_hash_address,
	},
	governance::GovernanceData,
};
use cardano_serialization_lib::{
	Address, JsError, NetworkIdKind, PrivateKey, Transaction, TransactionBody, TransactionBuilder,
	TransactionOutput, Value,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use serde::{Serialize, Serializer};
use sidechain_domain::{McTxHash, UtxoId, UtxoIndex, crypto::blake2b};

/// Successfull smart contracts offchain results in either transaction submission or creating transaction that has to be signed by the governance authorities
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MultiSigSmartContractResult {
	TransactionSubmitted(McTxHash),
	TransactionToSign(MultiSigTransactionData),
}

/// MultiSig transactions awaiting for signatures use temporary wallets where funds are stored until the transaction is signed and submitted.
/// This prevents payment utxo from being spend when the signatures for MultiSig are being collected.
#[derive(Clone, Debug, Serialize)]
pub struct MultiSigTransactionData {
	pub tx_name: String,
	pub temporary_wallet: TemporaryWalletData,
	#[serde(serialize_with = "serialize_as_conway_tx")]
	pub tx: Vec<u8>,
}

/// To be used only for manual re-claim of the funds if transaction has not been submitted
#[derive(Clone, Debug, Serialize)]
pub struct TemporaryWalletData {
	pub address: String,
	pub public_key_hash: String,
}

pub(crate) struct TemporaryWallet {
	pub address: cardano_serialization_lib::Address,
	pub private_key: CardanoPaymentSigningKey,
}

impl TemporaryWallet {
	pub(crate) fn address_bech32(&self) -> String {
		self.address.to_bech32(None).expect("to_bech32 is safe with None prefix")
	}
}

impl From<TemporaryWallet> for TemporaryWalletData {
	fn from(value: TemporaryWallet) -> Self {
		TemporaryWalletData {
			address: value.address_bech32(),
			public_key_hash: hex::encode(&value.private_key.to_pub_key_hash().0),
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

pub(crate) async fn fund_temporary_wallet<
	F: Fn(Costs, &TransactionContext) -> anyhow::Result<Transaction>,
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	make_tx: &F,
	payment_ctx: TransactionContext,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<TemporaryWallet> {
	let wallet = create_temporary_wallet(payment_ctx.network)?;
	let tx_to_estimate_costs = Costs::calculate_costs(|c| make_tx(c, &payment_ctx), client).await?;
	let value = estimate_required_value(tx_to_estimate_costs.body(), &payment_ctx)?;
	save_wallet_file(&wallet)?;
	transfer_to_temporary_wallet(payment_ctx, &wallet.address, &value, client, await_tx).await?;
	Ok(wallet)
}

fn create_temporary_wallet(network: NetworkIdKind) -> Result<TemporaryWallet, JsError> {
	let private_key = CardanoPaymentSigningKey(PrivateKey::generate_ed25519()?);
	let address = key_hash_address(&private_key.0.to_public().hash(), network);
	Ok(TemporaryWallet { private_key, address })
}

/// Estimates required value by subtracting change, fee and collateral from the sum of inputs.
/// Additional 5 ADA is subtracted, because multi assets present in inputs and change outputs,
/// affect also coin present in the change and can make the calculated required value too low.
fn estimate_required_value(
	tx: TransactionBody,
	ctx: &TransactionContext,
) -> Result<Value, JsError> {
	let mut change = Value::new(&0u32.into());
	for output in tx.outputs().into_iter() {
		if output.address() == ctx.change_address {
			change = change.checked_add(&output.amount())?;
		}
	}

	let mut total_input = Value::new(&0u32.into());
	for input in tx.inputs().into_iter() {
		if let Some(utxo) =
			ctx.payment_key_utxos.iter().find(|utxo| utxo.to_csl_tx_input() == *input)
		{
			total_input = total_input.checked_add(&utxo.to_csl()?.output().amount())?;
		}
	}

	total_input.clamped_sub(&change).checked_add(&Value::new(&5_000_000u32.into()))
}

fn save_wallet_file(wallet: &TemporaryWallet) -> Result<(), anyhow::Error> {
	let key_bytes = wallet.private_key.to_bytes();
	// CBOR wrappring the private key bytes. We don't have types to express this conveniently.
	let cbor_hex = format!("5820{}", hex::encode(key_bytes));
	let json = serde_json::json!({
		"type": "PaymentSigningKeyShelley_ed25519",
		"description": "Temporary wallet key generated for a MultiSigTransaction",
		"cborHex": cbor_hex
	});
	let file_name = format!("{}.skey", wallet.address_bech32());
	let file = std::fs::File::create(file_name)?;
	serde_json::to_writer_pretty(file, &json)?;
	Ok(())
}

async fn transfer_to_temporary_wallet<T: Transactions + QueryUtxoByUtxoId, A: AwaitTx>(
	payment_ctx: TransactionContext,
	address: &Address,
	value: &Value,
	client: &T,
	await_tx: &A,
) -> Result<(), anyhow::Error> {
	let mut funding_tx_builder = TransactionBuilder::new(&get_builder_config(&payment_ctx)?);
	funding_tx_builder.add_output(&TransactionOutput::new(address, value))?;
	let funding_tx = funding_tx_builder.balance_update_and_build(&payment_ctx)?;
	let tx_hash: [u8; 32] = blake2b(funding_tx.body().to_bytes().as_ref());
	log::info!(
		"Founding temporary wallet {} with {} in transaction: {}",
		&address.to_bech32(None)?,
		serde_json::to_string(&value)?,
		&hex::encode(tx_hash)
	);
	client.submit_transaction(&payment_ctx.sign(&funding_tx).to_bytes()).await?;
	await_tx.await_tx_output(client, UtxoId::new(tx_hash, 0)).await?;
	Ok(())
}

/// If the chain has real MultiSig governance it:
/// * creates a temporary wallet
/// * sends 5 ADA from the payment wallet (subject of change)
/// * creates a transaction that would be paid from the temporary wallet, signed by both wallets.
///
/// If the chain has single key MultiSig governance it creates and submits transaction paid by and signed by the payment wallet.
pub(crate) async fn submit_or_create_tx_to_sign<F, T, A>(
	governance_data: &GovernanceData,
	payment_ctx: TransactionContext,
	make_tx: F,
	tx_name: &str,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult>
where
	F: Fn(Costs, &TransactionContext) -> anyhow::Result<Transaction>,
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
{
	Ok(if governance_data.policy.is_single_key_policy_for(&payment_ctx.payment_key_hash()) {
		MultiSigSmartContractResult::TransactionSubmitted(
			submit_single_governance_key_tx(payment_ctx, make_tx, tx_name, client, await_tx)
				.await?,
		)
	} else {
		MultiSigSmartContractResult::TransactionToSign(
			create_transaction_to_sign(payment_ctx, make_tx, tx_name, client, await_tx).await?,
		)
	})
}

async fn submit_single_governance_key_tx<F, T, A>(
	payment_ctx: TransactionContext,
	make_tx: F,
	tx_name: &str,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<McTxHash>
where
	F: Fn(Costs, &TransactionContext) -> anyhow::Result<Transaction>,
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
{
	let tx = Costs::calculate_costs(|c| make_tx(c, &payment_ctx), client).await?;
	let signed_tx = payment_ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Submit '{}' transaction request failed: {}, bytes: {}",
			tx_name,
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = McTxHash(res.transaction.id);
	log::info!("'{}' transaction submitted: {}", tx_name, hex::encode(tx_id.0));
	await_tx
		.await_tx_output(
			client,
			UtxoId { tx_hash: McTxHash(res.transaction.id), index: UtxoIndex(0) },
		)
		.await?;
	Ok(tx_id)
}

async fn create_transaction_to_sign<F, T, A>(
	payment_ctx: TransactionContext,
	make_tx: F,
	tx_name: &str,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<MultiSigTransactionData>
where
	F: Fn(Costs, &TransactionContext) -> anyhow::Result<Transaction>,
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
{
	let original_ctx = payment_ctx.clone();
	let temporary_wallet = fund_temporary_wallet(&make_tx, payment_ctx, client, await_tx)
		.await
		.map_err(|e| {
			anyhow::anyhow!("Failed to create temporary wallet for '{}': {}", tx_name, e)
		})?;
	let temp_wallet_ctx =
		TransactionContext::for_payment_key(&temporary_wallet.private_key, client)
			.await?
			.with_change_address(&original_ctx.change_address);
	let tx =
		Costs::calculate_costs(|c| make_tx(c, &temp_wallet_ctx), client)
			.await
			.map_err(|e| {
				anyhow::anyhow!(
					"Failed to create '{}' transaction using temporary wallet: {}",
					tx_name,
					e
				)
			})?;
	let signed_tx_by_caller = original_ctx.sign(&tx);
	let signed_tx = temp_wallet_ctx.sign(&signed_tx_by_caller);
	Ok(MultiSigTransactionData {
		tx_name: tx_name.to_owned(),
		temporary_wallet: temporary_wallet.into(),
		tx: signed_tx.to_bytes(),
	})
}
