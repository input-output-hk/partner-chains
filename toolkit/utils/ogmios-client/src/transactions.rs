//! Requests to evalute and submit transactions via Ogmios`.

use crate::{types::OgmiosTx, ByNameParamsBuilder, OgmiosClient, OgmiosClientError};
use serde::Deserialize;

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
	) -> Result<Vec<OgmiosEvaluateTransactionResponse>, OgmiosClientError<EvaluateTransactionError>>;

	/// Submits a signed transaction.
	///
	/// Parameters:
	/// - `tx_bytes: &[u8]` - CBOR-serialized signed transaction
	#[allow(async_fn_in_trait)]
	async fn submit_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<SubmitTransactionResponse, OgmiosClientError<SubmitTransactionError>>;
}

#[derive(Deserialize, Clone, Debug)]
pub enum EvaluateTransactionError {
	EvaluateError(crate::generated::EvaluateTransactionFailure),
	DeserialisationError(crate::generated::DeserialisationFailure),
}

impl std::fmt::Display for EvaluateTransactionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::EvaluateError(e) => write!(f, "{:?}", e),
			Self::DeserialisationError(e) => write!(f, "{:?}", e),
		}
	}
}

#[derive(Deserialize, Clone, Debug)]
pub enum SubmitTransactionError {
	SubmitError(crate::generated::SubmitTransactionFailure),
	DeserialisationError(crate::generated::DeserialisationFailure),
}

impl std::fmt::Display for SubmitTransactionError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::SubmitError(e) => write!(f, "{:?}", e),
			Self::DeserialisationError(e) => write!(f, "{:?}", e),
		}
	}
}

impl<T: OgmiosClient> Transactions for T {
	async fn evaluate_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<Vec<OgmiosEvaluateTransactionResponse>, OgmiosClientError<EvaluateTransactionError>>
	{
		let params = ByNameParamsBuilder::new()
			.insert("transaction", serde_json::json!({"cbor": hex::encode(tx_bytes)}))?
			.insert("additionalUtxo", serde_json::json!([]))?
			.build();
		self.request("evaluateTransaction", params).await
	}

	async fn submit_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<SubmitTransactionResponse, OgmiosClientError<SubmitTransactionError>> {
		let params = ByNameParamsBuilder::new()
			.insert("transaction", serde_json::json!({"cbor": hex::encode(tx_bytes)}))?
			.build();
		self.request("submitTransaction", params).await
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct OgmiosEvaluateTransactionResponse {
	pub validator: OgmiosValidatorIndex,
	pub budget: OgmiosBudget,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
pub struct OgmiosValidatorIndex {
	pub index: u32,
	pub purpose: String,
}

impl OgmiosValidatorIndex {
	pub fn new(index: u32, purpose: &str) -> Self {
		Self { index, purpose: purpose.into() }
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
pub struct OgmiosBudget {
	pub memory: u64,
	pub cpu: u64,
}

impl OgmiosBudget {
	pub fn new(memory: u64, cpu: u64) -> Self {
		Self { memory, cpu }
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
pub struct SubmitTransactionResponse {
	pub transaction: OgmiosTx,
}
