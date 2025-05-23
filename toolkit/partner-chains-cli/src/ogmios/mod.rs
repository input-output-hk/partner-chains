use crate::IOContext;
use anyhow::anyhow;
use ogmios_client::{
	jsonrpsee::client_for_url, query_ledger_state::QueryLedgerState, query_network::QueryNetwork,
	types::OgmiosUtxo,
};
use sidechain_domain::NetworkType;
use std::time::Duration;

pub(crate) mod config;

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
	pub network: NetworkType,
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
	tokio_runtime.block_on(async {
		let client = client_for_url(addr, Some(Duration::from_secs(180)))
			.await
			.map_err(|e| anyhow::anyhow!("Failed to connect to Ogmios at {} with: {}", addr, e))?;
		match req {
			OgmiosRequest::QueryLedgerStateEraSummaries => {
				let era_summaries = client.era_summaries().await.map_err(|e| anyhow::anyhow!(e))?;
				let era_summaries = era_summaries.into_iter().map(From::from).collect();
				Ok(OgmiosResponse::QueryLedgerStateEraSummaries(era_summaries))
			},
			OgmiosRequest::QueryNetworkShelleyGenesis => {
				let shelley_genesis =
					client.shelley_genesis_configuration().await.map_err(|e| anyhow::anyhow!(e))?;
				let shelley_genesis = ShelleyGenesisConfiguration::try_from(shelley_genesis)?;
				Ok(OgmiosResponse::QueryNetworkShelleyGenesis(shelley_genesis))
			},
			OgmiosRequest::QueryUtxo { address } => {
				let utxos = client.query_utxos(&[address]).await.map_err(|e| anyhow::anyhow!(e))?;
				Ok(OgmiosResponse::QueryUtxo(utxos))
			},
		}
	})
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
			network: shelley_genesis.network,
			security_parameter: shelley_genesis.security_parameter,
			active_slots_coefficient,
			epoch_length: shelley_genesis.epoch_length,
			slot_length_millis: shelley_genesis.slot_length.milliseconds.into(),
			start_time: shelley_genesis.start_time.unix_timestamp().try_into()?,
		})
	}
}

pub(crate) fn get_shelley_config<C: IOContext>(
	addr: &str,
	context: &C,
) -> anyhow::Result<ShelleyGenesisConfiguration> {
	let response = context.ogmios_rpc(addr, OgmiosRequest::QueryNetworkShelleyGenesis)?;
	match response {
		OgmiosResponse::QueryNetworkShelleyGenesis(shelley_config) => Ok(shelley_config),
		other => Err(anyhow::anyhow!(format!(
			"Unexpected response from Ogmios when quering for shelley genesis configuration: {other:?}"
		))),
	}
}

#[cfg(test)]
pub(crate) mod test_values {
	use crate::ogmios::EpochParameters;

	use super::ShelleyGenesisConfiguration;
	use super::{EpochBoundary, EraSummary};
	use sidechain_domain::NetworkType;

	pub(crate) fn preprod_eras_summaries() -> Vec<EraSummary> {
		vec![
			EraSummary {
				start: EpochBoundary { time_seconds: 0, slot: 0, epoch: 0 },
				parameters: EpochParameters { epoch_length: 21600, slot_length_millis: 20000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 1728000, slot: 86400, epoch: 4 },
				parameters: EpochParameters { epoch_length: 432000, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 2160000, slot: 518400, epoch: 5 },
				parameters: EpochParameters { epoch_length: 432000, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 2592000, slot: 950400, epoch: 6 },
				parameters: EpochParameters { epoch_length: 432000, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 3024000, slot: 1382400, epoch: 7 },
				parameters: EpochParameters { epoch_length: 432000, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 5184000, slot: 3542400, epoch: 12 },
				parameters: EpochParameters { epoch_length: 432000, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 5184000, slot: 3542400, epoch: 12 },
				parameters: EpochParameters { epoch_length: 432000, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 70416000, slot: 68774400, epoch: 163 },
				parameters: EpochParameters { epoch_length: 432000, slot_length_millis: 1000 },
			},
		]
	}

	pub(crate) fn preprod_shelley_config() -> ShelleyGenesisConfiguration {
		ShelleyGenesisConfiguration {
			security_parameter: 2160,
			active_slots_coefficient: 0.05,
			epoch_length: 432000,
			slot_length_millis: 1000,
			start_time: 1654041600,
			network: NetworkType::Testnet,
		}
	}

	pub(crate) fn preview_eras_summaries() -> Vec<EraSummary> {
		vec![
			EraSummary {
				start: EpochBoundary { time_seconds: 0, slot: 0, epoch: 0 },
				parameters: EpochParameters { epoch_length: 4320, slot_length_millis: 20000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 0, slot: 0, epoch: 0 },
				parameters: EpochParameters { epoch_length: 86400, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 0, slot: 0, epoch: 0 },
				parameters: EpochParameters { epoch_length: 86400, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 0, slot: 0, epoch: 0 },
				parameters: EpochParameters { epoch_length: 86400, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 0, slot: 0, epoch: 0 },
				parameters: EpochParameters { epoch_length: 86400, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 259200, slot: 259200, epoch: 3 },
				parameters: EpochParameters { epoch_length: 86400, slot_length_millis: 1000 },
			},
			EraSummary {
				start: EpochBoundary { time_seconds: 55814400, slot: 55814400, epoch: 646 },
				parameters: EpochParameters { epoch_length: 86400, slot_length_millis: 1000 },
			},
		]
	}

	pub(crate) fn preview_shelley_config() -> ShelleyGenesisConfiguration {
		ShelleyGenesisConfiguration {
			security_parameter: 432,
			active_slots_coefficient: 0.05,
			epoch_length: 86400,
			slot_length_millis: 1000,
			start_time: 1666656000,
			network: NetworkType::Testnet,
		}
	}
}
