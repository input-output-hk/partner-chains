/// This module provides a high-level API for interacting with the Ogmios API.
/// It uses jsonrpsee to communicate with the Ogmios server. It should be either replaced or improved to not lose Ogmios error messages that are very helpful.
use anyhow::anyhow;
use jsonrpsee::{
	core::{client::ClientT, params::ObjectParams},
	http_client::HttpClient,
};
use pallas_addresses::ShelleyAddress;
use serde::{Deserialize, Serialize};

pub async fn query_utxos(
	addr: &ShelleyAddress,
	client: &HttpClient,
) -> Result<Vec<OgmiosUtxo>, anyhow::Error> {
	let mut params = ObjectParams::new();
	let addr = addr.to_bech32().unwrap();
	params.insert("addresses", vec![addr.clone()]).unwrap();
	client
		.request("queryLedgerState/utxo", params)
		.await
		.map_err(|e| anyhow!("Couldn't get UTXOs of {}, because of {}", addr, e.to_string()))
}

pub async fn query_protocol_parameters(
	client: &HttpClient,
) -> Result<ProtocolParametersResponse, anyhow::Error> {
	client
		.request("queryLedgerState/protocolParameters", ObjectParams::new())
		.await
		.map_err(|e| anyhow!("Couldn't get protocol parameters, because of {}", e.to_string()))
}

pub async fn submit_tx(
	tx_bytes: &[u8],
	client: &HttpClient,
) -> Result<serde_json::Value, anyhow::Error> {
	let tx_bytes_hex = hex::encode(tx_bytes);
	println!("submit tx:\n{}", tx_bytes_hex);
	let mut params = ObjectParams::new();
	params.insert("transaction", serde_json::json!({"cbor": tx_bytes_hex})).unwrap();
	client
		.request("submitTransaction", params)
		.await
		.map_err(|e| anyhow!("Couldn't submit tx {}", e.to_string()))
}

pub async fn evalutate_tx(
	tx_bytes: &[u8],
	client: &HttpClient,
) -> Result<Vec<OgmiosEvaluateTransactionResponse>, anyhow::Error> {
	let tx_bytes_hex = hex::encode(tx_bytes);
	let mut params = ObjectParams::new();
	println!("evaluate tx\n{}", tx_bytes_hex);
	params.insert("transaction", serde_json::json!({"cbor": tx_bytes_hex})).unwrap();
	params.insert("additionalUtxo", serde_json::Value::Array(vec![])).unwrap();
	client
		.request("evaluateTransaction", params)
		.await
		.map_err(|e| anyhow!("Couldn't evaluate tx {}", e.to_string()))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TxParam {
	cbor: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OgmiosUtxoQueryParams {
	addresses: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OgmiosUtxo {
	pub transaction: OgmiosTx,
	pub index: u32,
	pub address: String,
	pub value: OgmiosValue,
	pub datum: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OgmiosTx {
	pub id: String,
}

// TODO: add native tokens
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OgmiosValue {
	Ada { lovelace: u64 },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParametersResponse {
	pub min_fee_coefficient: u32,
	pub min_fee_constant: OgmiosValue,
	pub stake_pool_deposit: OgmiosValue,
	pub stake_credential_deposit: OgmiosValue,
	pub max_value_size: OgmiosBytesSize,
	pub max_transaction_size: OgmiosBytesSize,
	pub plutus_cost_models: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TipResponse {
	pub slot: u64,
	pub id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OgmiosBytesSize {
	pub bytes: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OgmiosEvaluateTransactionResponse {
	pub validator: OgmiosValidatorIndex,
	pub budget: OgmiosBudget,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OgmiosValidatorIndex {
	pub index: u32,
	pub purpose: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OgmiosBudget {
	pub memory: u64,
	pub cpu: u64,
}
