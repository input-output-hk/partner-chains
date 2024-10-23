//! Queries that start with `queryLedgerState/`.

use std::collections::HashMap;

use crate::{
	types::{OgmiosBytesSize, OgmiosUtxo, OgmiosValue, SlotLength, TimeSeconds},
	ByNameParamsBuilder, OgmiosClient, OgmiosClientError, OgmiosParams,
};
use serde::Deserialize;

pub trait QueryLedgerState: OgmiosClient {
	#[allow(async_fn_in_trait)]
	async fn era_summaries(&self) -> Result<Vec<EraSummary>, OgmiosClientError> {
		self.request("queryLedgerState/eraSummaries", OgmiosParams::empty_positional())
			.await
	}

	#[allow(async_fn_in_trait)]
	/// Parameters:
	/// - `addresses`: bech32 address to query
	async fn query_utxos(
		&self,
		addresses: &[String],
	) -> Result<Vec<OgmiosUtxo>, OgmiosClientError> {
		let params = ByNameParamsBuilder::new().insert("addresses", addresses)?.build();
		self.request("queryLedgerState/utxo", params).await
	}

	#[allow(async_fn_in_trait)]
	async fn query_protocol_parameters(
		&self,
	) -> Result<ProtocolParametersResponse, OgmiosClientError> {
		self.request("queryLedgerState/protocolParameters", OgmiosParams::empty_by_name())
			.await
	}
}

impl<T: OgmiosClient> QueryLedgerState for T {}

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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolParametersResponse {
	pub min_fee_coefficient: u32,
	pub min_fee_constant: OgmiosValue,
	pub stake_pool_deposit: OgmiosValue,
	pub stake_credential_deposit: OgmiosValue,
	pub max_value_size: OgmiosBytesSize,
	pub max_transaction_size: OgmiosBytesSize,
	pub plutus_cost_models: HashMap<String, Vec<i128>>,
}
