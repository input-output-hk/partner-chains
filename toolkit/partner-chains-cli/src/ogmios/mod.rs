use crate::config::CardanoNetwork;
use anyhow::anyhow;
use jsonrpsee::http_client::HttpClient;
use ogmios_client::{
	query_ledger_state::QueryLedgerState,
	query_network::QueryNetwork,
	types::OgmiosUtxo,
};

#[derive(Debug, Eq, PartialEq)]
pub enum OgmiosRequest {
	QueryLedgerStateEraSummaries,
	QueryNetworkShelleyGenesis,
	QueryUtxo { address: String },
}

#[derive(Debug)]
pub enum OgmiosResponse {
	QueryLedgerStateEraSummaries(Vec<EraSummary>),
	QueryNetworkShelleyGenesis(ShelleyGenesisConfiguration),
	QueryUtxo(Vec<OgmiosUtxo>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EraSummary {
	pub start: EpochBoundary,
	pub parameters: EpochParameters,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpochBoundary {
	pub time_seconds: u64,
	pub slot: u64,
	pub epoch: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpochParameters {
	pub epoch_length: u32,
	pub slot_length_millis: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShelleyGenesisConfiguration {
	pub network: CardanoNetwork,
	pub security_parameter: u32,
	pub active_slots_coefficient: f64,
	pub epoch_length: u32,
	pub slot_length_millis: u64,
	// Seconds since UNIX epoch
	pub start_time: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Utxo {
	pub tx_id: [u8; 32],
	pub index: u32,
	pub value: UtxoValue,
}

type AssetName = Vec<u8>;
type PolicyId = [u8; 28];

#[derive(Clone, Debug, PartialEq)]
pub struct UtxoValue {
	pub lovelace: u64,
	pub assets: Vec<(PolicyId, Vec<(AssetName, i128)>)>,
}

pub fn ogmios_request(addr: &str, req: OgmiosRequest) -> anyhow::Result<OgmiosResponse> {
	let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
	let client = HttpClient::builder().build(addr).map_err(|e| anyhow::anyhow!(e))?;
	match req {
		OgmiosRequest::QueryLedgerStateEraSummaries => {
			let era_summaries =
				tokio_runtime.block_on(client.era_summaries()).map_err(|e| anyhow::anyhow!(e))?;
			let era_summaries = era_summaries.into_iter().map(From::from).collect();
			Ok(OgmiosResponse::QueryLedgerStateEraSummaries(era_summaries))
		},
		OgmiosRequest::QueryNetworkShelleyGenesis => {
			let shelley_genesis = tokio_runtime
				.block_on(client.shelley_genesis_configuration())
				.map_err(|e| anyhow::anyhow!(e))?;
			let shelley_genesis = ShelleyGenesisConfiguration::try_from(shelley_genesis)?;
			Ok(OgmiosResponse::QueryNetworkShelleyGenesis(shelley_genesis))
		},
		OgmiosRequest::QueryUtxo { address } => {
			let utxos = tokio_runtime
				.block_on(client.query_utxos(&[address]))
				.map_err(|e| anyhow::anyhow!(e))?;
			Ok(OgmiosResponse::QueryUtxo(utxos))
		},
	}
}

impl From<ogmios_client::query_ledger_state::EraSummary> for EraSummary {
	fn from(era_summary: ogmios_client::query_ledger_state::EraSummary) -> Self {
		Self {
			start: From::from(era_summary.start),
			parameters: From::from(era_summary.parameters),
		}
	}
}

impl From<ogmios_client::query_ledger_state::EpochBoundary> for EpochBoundary {
	fn from(epoch_boundary: ogmios_client::query_ledger_state::EpochBoundary) -> Self {
		Self {
			time_seconds: epoch_boundary.time.seconds,
			slot: epoch_boundary.slot,
			epoch: epoch_boundary.epoch,
		}
	}
}

impl From<ogmios_client::query_ledger_state::EpochParameters> for EpochParameters {
	fn from(epoch_boundary: ogmios_client::query_ledger_state::EpochParameters) -> Self {
		Self {
			epoch_length: epoch_boundary.epoch_length,
			slot_length_millis: epoch_boundary.slot_length.milliseconds.into(),
		}
	}
}

impl TryFrom<ogmios_client::query_network::ShelleyGenesisConfigurationResponse>
	for ShelleyGenesisConfiguration
{
	type Error = anyhow::Error;

	fn try_from(
		shelley_genesis: ogmios_client::query_network::ShelleyGenesisConfigurationResponse,
	) -> Result<Self, Self::Error> {
		let active_slots_coefficient = TryFrom::try_from(shelley_genesis.active_slots_coefficient)
			.map_err(|_| anyhow!("Cannot convert active_slots_coefficient"))?;
		Ok(Self {
			network: shelley_genesis.network.into(),
			security_parameter: shelley_genesis.security_parameter,
			active_slots_coefficient,
			epoch_length: shelley_genesis.epoch_length,
			slot_length_millis: shelley_genesis.slot_length.milliseconds.into(),
			start_time: shelley_genesis.start_time.unix_timestamp().try_into()?,
		})
	}
}
