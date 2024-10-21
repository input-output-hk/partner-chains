//! Requests to evalute and submit transactions via Ogmios`.

use crate::{ByNameParamsBuilder, OgmiosClient, OgmiosClientError};
use serde::{Deserialize, Deserializer};

pub trait Transactions: OgmiosClient {
	/// Evaluates a transaction.
	///
	/// Does not support additional UTXO inputs yet.
	///
	/// Parameters:
	/// - `tx_bytes: &[u8]` - CBOR-serialized transaction
	#[allow(async_fn_in_trait)]
	async fn evalute_transaction(
		&self,
		tx_bytes: &[u8],
	) -> Result<Vec<OgmiosEvaluateTransactionResponse>, OgmiosClientError> {
		let params = ByNameParamsBuilder::new()
			.insert("transaction", serde_json::json!({"cbor": hex::encode(tx_bytes)}))?
			.insert("additionalUtxo", serde_json::json!([]))?
			.build();
		self.request("evaluateTransaction", params).await
	}

	/// Submits a signed transaction.
	///
	/// Parameters:
	/// - `tx_bytes: &[u8]` - CBOR-serialized signed transaction
	#[allow(async_fn_in_trait)]
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

impl<T: OgmiosClient> Transactions for T {}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OgmiosEvaluateTransactionResponse {
	pub validator: OgmiosValidatorIndex,
	pub budget: OgmiosBudget,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OgmiosValidatorIndex {
	pub index: u32,
	pub purpose: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct OgmiosBudget {
	pub memory: u64,
	pub cpu: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SubmitTransactionResponse {
	#[serde(deserialize_with = "parse_tx_id")]
	pub id: [u8; 32],
}

fn parse_tx_id<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
	D: Deserializer<'de>,
{
	let buf = String::deserialize(deserializer)?;
	let vec = hex::decode(buf).map_err(serde::de::Error::custom)?;
	TryFrom::try_from(vec)
		.map_err(|e| serde::de::Error::custom(format!("{} has invalid size", hex::encode(e))))
}
