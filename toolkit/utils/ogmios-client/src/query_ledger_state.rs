//! Queries that start with `queryLedgerState/`.

use crate::{
	types::{OgmiosBytesSize, OgmiosUtxo, OgmiosValue, SlotLength, TimeSeconds},
	ByNameParamsBuilder, OgmiosClient, OgmiosClientError, OgmiosParams,
};
use serde::Deserialize;

pub trait QueryLedgerState {
	#[allow(async_fn_in_trait)]
	async fn era_summaries(&self) -> Result<Vec<EraSummary>, OgmiosClientError>;

	#[allow(async_fn_in_trait)]
	/// Parameters:
	/// - `addresses`: bech32 address to query
	async fn query_utxos(&self, addresses: &[String])
		-> Result<Vec<OgmiosUtxo>, OgmiosClientError>;

	#[allow(async_fn_in_trait)]
	async fn query_protocol_parameters(
		&self,
	) -> Result<ProtocolParametersResponse, OgmiosClientError>;
}

pub trait QueryUtxoByUtxoId {
	#[allow(async_fn_in_trait)]
	/// Query for a single UTXO by transaction hash and output index.
	/// Warning: it does not return datum, datumHash, nor script fields.
	/// Parameters:
	/// - `tx`: query for output of this transaction
	/// - `index`: query for output with this index
	async fn query_utxo_by_id(
		&self,
		utxo: sidechain_domain::UtxoId,
	) -> Result<Option<OgmiosUtxo>, OgmiosClientError>;
}

impl<T: OgmiosClient> QueryLedgerState for T {
	async fn era_summaries(&self) -> Result<Vec<EraSummary>, OgmiosClientError> {
		self.request("queryLedgerState/eraSummaries", OgmiosParams::empty_positional())
			.await
	}

	async fn query_utxos(
		&self,
		addresses: &[String],
	) -> Result<Vec<OgmiosUtxo>, OgmiosClientError> {
		let params = ByNameParamsBuilder::new().insert("addresses", addresses)?.build();
		self.request("queryLedgerState/utxo", params).await
	}

	async fn query_protocol_parameters(
		&self,
	) -> Result<ProtocolParametersResponse, OgmiosClientError> {
		self.request("queryLedgerState/protocolParameters", OgmiosParams::empty_by_name())
			.await
	}
}

impl<T: OgmiosClient> QueryUtxoByUtxoId for T {
	async fn query_utxo_by_id(
		&self,
		utxo: sidechain_domain::UtxoId,
	) -> Result<Option<OgmiosUtxo>, OgmiosClientError> {
		let reference = serde_json::json!({
			"transaction": {"id": hex::encode(utxo.tx_hash.0)},
			"index": utxo.index.0,
		});
		let params =
			ByNameParamsBuilder::new().insert("outputReferences", vec![reference])?.build();
		// Expect at most one output, because it is a single output reference query.
		let utxos: Vec<OgmiosUtxo> = self.request("queryLedgerState/utxo", params).await?;
		Ok(utxos.first().cloned())
	}
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EraSummary {
	pub start: EpochBoundary,
	pub end: EpochBoundary,
	pub parameters: EpochParameters,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct EpochBoundary {
	pub time: TimeSeconds,
	pub slot: u64,
	pub epoch: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EpochParameters {
	pub epoch_length: u32,
	pub slot_length: SlotLength,
	pub safe_zone: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceScriptsCosts {
	pub base: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParametersResponse {
	pub min_fee_coefficient: u32,
	pub min_fee_constant: OgmiosValue,
	pub stake_pool_deposit: OgmiosValue,
	pub stake_credential_deposit: OgmiosValue,
	pub max_value_size: OgmiosBytesSize,
	pub max_transaction_size: OgmiosBytesSize,
	pub min_utxo_deposit_coefficient: u64,
	pub script_execution_prices: ScriptExecutionPrices,
	pub plutus_cost_models: PlutusCostModels,
	pub max_collateral_inputs: u32,
	pub collateral_percentage: u32,
	pub min_fee_reference_scripts: ReferenceScriptsCosts,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Default)]
pub struct ScriptExecutionPrices {
	#[serde(deserialize_with = "crate::types::parse_fraction_ratio_u64")]
	pub memory: fraction::Ratio<u64>,
	#[serde(deserialize_with = "crate::types::parse_fraction_ratio_u64")]
	pub cpu: fraction::Ratio<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Default)]
pub struct PlutusCostModels {
	#[serde(rename = "plutus:v1")]
	pub plutus_v1: Vec<i128>,
	#[serde(rename = "plutus:v2")]
	pub plutus_v2: Vec<i128>,
	#[serde(rename = "plutus:v3")]
	pub plutus_v3: Vec<i128>,
}
