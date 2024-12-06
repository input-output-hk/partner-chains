//! Requests to evalute and submit transactions via Ogmios`.

use crate::{types::OgmiosTx, ByNameParamsBuilder, OgmiosClient, OgmiosClientError};
use cardano_serialization_lib::ExUnits;
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
pub struct OgmiosEvaluateTransactionResponse {
	pub validator: OgmiosValidatorIndex,
	pub budget: OgmiosBudget,
}

impl From<OgmiosEvaluateTransactionResponse> for ExUnits {
	fn from(resp: OgmiosEvaluateTransactionResponse) -> Self {
		resp.budget.into()
	}
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

impl From<OgmiosBudget> for ExUnits {
	fn from(v: OgmiosBudget) -> Self {
		ExUnits::new(&v.memory.into(), &v.cpu.into())
	}
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Default)]
pub struct SubmitTransactionResponse {
	pub transaction: OgmiosTx,
}
