//! Queries that start with `queryLedgerState/`.

use crate::{
	ByNameParamsBuilder, OgmiosClient, OgmiosClientError, OgmiosParams,
	types::{OgmiosBytesSize, OgmiosUtxo, OgmiosValue, SlotLength, TimeSeconds},
};
use serde::Deserialize;

/// Trait that defines the methods for querying the Cardano ledger state.
pub trait QueryLedgerState {
	#[allow(async_fn_in_trait)]
	/// Returns the slot number of the most recent block in the blockchain.
	async fn get_tip(&self) -> Result<OgmiosTip, OgmiosClientError>;

	#[allow(async_fn_in_trait)]
	/// Returns a list of era summaries.
	async fn era_summaries(&self) -> Result<Vec<EraSummary>, OgmiosClientError>;

	#[allow(async_fn_in_trait)]
	/// Parameters:
	/// - `addresses`: bech32 address to query
	async fn query_utxos(&self, addresses: &[String])
	-> Result<Vec<OgmiosUtxo>, OgmiosClientError>;

	#[allow(async_fn_in_trait)]
	/// Returns the current protocol parameters.
	async fn query_protocol_parameters(
		&self,
	) -> Result<ProtocolParametersResponse, OgmiosClientError>;
}

/// Trait that defines the methods for querying a single UTXO by transaction hash and output index.
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
	async fn get_tip(&self) -> Result<OgmiosTip, OgmiosClientError> {
		self.request("queryLedgerState/tip", OgmiosParams::empty_positional()).await
	}

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
/// Represents a summary of a known era.
pub struct EraSummary {
	/// The start boundary of the era.
	pub start: EpochBoundary,
	/// The end boundary of the era.
	pub end: EpochBoundary,
	/// The parameters of the era.
	pub parameters: EpochParameters,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
/// Represents the boundary of an epoch.
pub struct EpochBoundary {
	pub time: TimeSeconds,
	pub slot: u64,
	pub epoch: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
/// Represents the parameters of an epoch.
pub struct EpochParameters {
	/// The length of the epoch in slots.
	pub epoch_length: u32,
	/// The length of a slot in milliseconds.
	pub slot_length: SlotLength,
	/// Number of slots from the tip of the ledger in which it is guaranteed that no hard fork can take place.
	pub safe_zone: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
/// Represents the cost of reference scripts.
pub struct ReferenceScriptsCosts {
	/// The base cost of a reference script.
	pub base: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
/// Represents the protocol parameters.
pub struct ProtocolParametersResponse {
	/// Additional transaction fee per byte of data (in lovelace). Also called minFeeA.
	pub min_fee_coefficient: u32,
	/// Base transaction fee (in lovelace). Also called minFeeB.
	pub min_fee_constant: OgmiosValue,
	/// Amount of lovelace required to register a stake pool.
	pub stake_pool_deposit: OgmiosValue,
	/// Amount of lovelace required to register a stake credential.
	pub stake_credential_deposit: OgmiosValue,
	/// Maximum size limit for the value field in transaction outputs.
	pub max_value_size: OgmiosBytesSize,
	/// Maximum size limit for the transaction body.
	pub max_transaction_size: OgmiosBytesSize,
	/// Additional transaction fee per byte of output data (in lovelace). Also called coinsPerUTxOWord or coinsPerUTxOByte
	pub min_utxo_deposit_coefficient: u64,
	/// Pricing for Plutus script execution resources.
	pub script_execution_prices: ScriptExecutionPrices,
	/// Cost models for different Plutus language versions.
	pub plutus_cost_models: PlutusCostModels,
	/// Maximum number of collateral inputs that can be included in a transaction.
	pub max_collateral_inputs: u32,
	/// Percentage of fee that is used as collateral for a failed transaction.
	pub collateral_percentage: u32,
	/// Cost of reference scripts.
	pub min_fee_reference_scripts: ReferenceScriptsCosts,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Default)]
/// Represents the pricing for Plutus script execution resources.
pub struct ScriptExecutionPrices {
	#[serde(deserialize_with = "crate::types::parse_fraction_ratio_u64")]
	/// Fee per memory unit.
	pub memory: fraction::Ratio<u64>,
	#[serde(deserialize_with = "crate::types::parse_fraction_ratio_u64")]
	/// Fee per CPU unit.
	pub cpu: fraction::Ratio<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Default)]
/// Represents the cost models for different Plutus language versions.
pub struct PlutusCostModels {
	#[serde(rename = "plutus:v1")]
	/// Cost model for Plutus v1.
	pub plutus_v1: Vec<i128>,
	#[serde(rename = "plutus:v2")]
	/// Cost model for Plutus v2.
	pub plutus_v2: Vec<i128>,
	#[serde(rename = "plutus:v3")]
	/// Cost model for Plutus v3.
	pub plutus_v3: Vec<i128>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Default)]
/// Represents the tip of the ledger.
pub struct OgmiosTip {
	/// The slot number of the most recent block in the blockchain.
	pub slot: u64,
}
