//! Requests to evalute and submit transactions via Ogmios`.

use crate::{ByNameParamsBuilder, OgmiosClient, OgmiosClientError, types::OgmiosTx};
use serde::Deserialize;

/// Trait that defines the methods for evaluating and submitting transactions via Ogmios.
pub trait Transactions {
	/// Evaluates a transaction.
	///
	/// Does not support additional UTXO inputs yet.
	///
	/// Parameters:
	/// - `tx_bytes: &[u8]` - CBOR-serialized transaction
	#[allow(async_fn_in_trait)]
	async fn evaluate_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<Vec<OgmiosEvaluateTransactionResponse>, OgmiosClientError>;

	/// Submits a signed transaction.
	///
	/// Parameters:
	/// - `tx_bytes: &[u8]` - CBOR-serialized signed transaction
	#[allow(async_fn_in_trait)]
	async fn submit_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<SubmitTransactionResponse, OgmiosClientError>;
}

impl<T: OgmiosClient> Transactions for T {
	async fn evaluate_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<Vec<OgmiosEvaluateTransactionResponse>, OgmiosClientError> {
		let params = ByNameParamsBuilder::new()
			.insert("transaction", serde_json::json!({"cbor": hex::encode(tx_bytes)}))?
			.insert("additionalUtxo", serde_json::json!([]))?
			.build();
		self.request("evaluateTransaction", params).await
	}

	async fn submit_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<SubmitTransactionResponse, OgmiosClientError> {
		let params = ByNameParamsBuilder::new()
			.insert("transaction", serde_json::json!({"cbor": hex::encode(tx_bytes)}))?
			.build();
		self.request("submitTransaction", params).await
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
/// Represents the response from evaluating a transaction.
pub struct OgmiosEvaluateTransactionResponse {
	/// The smart contract index.
	pub validator: OgmiosValidatorIndex,
	/// The costs of smart contract execution.
	pub budget: OgmiosBudget,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
/// Represents the smart contract index.
pub struct OgmiosValidatorIndex {
	/// The index of the smart contract.
	pub index: u32,
	/// The purpose of the smart contract. Values allowed are:
	/// * spend
	/// * mint
	/// * publish
	/// * withdraw
	/// * vote
	/// * propose
	pub purpose: String,
}

impl OgmiosValidatorIndex {
	/// Creates a new smart contract index.
	pub fn new(index: u32, purpose: &str) -> Self {
		Self { index, purpose: purpose.into() }
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
/// Represents the budget for smart contract execution.
pub struct OgmiosBudget {
	/// The memory budget.
	pub memory: u64,
	/// The CPU budget.
	pub cpu: u64,
}

impl OgmiosBudget {
	/// Creates a new budget.
	pub fn new(memory: u64, cpu: u64) -> Self {
		Self { memory, cpu }
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
/// Represents the response from submitting a transaction.
pub struct SubmitTransactionResponse {
	/// The transaction hash.
	pub transaction: OgmiosTx,
}
