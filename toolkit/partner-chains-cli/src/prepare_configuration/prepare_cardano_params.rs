use crate::config::{CardanoNetwork, CardanoParameters, ServiceConfig};
use crate::io::IOContext;
use crate::ogmios::{EraSummary, OgmiosRequest, OgmiosResponse, ShelleyGenesisConfiguration};

pub fn prepare_cardano_params<C: IOContext>(
	ogmios_config: &ServiceConfig,
	context: &C,
) -> anyhow::Result<CardanoParameters> {
	let addr = format!("{}", ogmios_config);
	let eras_summaries = get_eras_summaries(&addr, context)?;
	let shelley_config = get_shelley_config(&addr, context)?;
	caradano_parameters(eras_summaries, shelley_config)
}

fn get_eras_summaries<C: IOContext>(addr: &str, context: &C) -> anyhow::Result<Vec<EraSummary>> {
	let eras_summaries = context.ogmios_rpc(addr, OgmiosRequest::QueryLedgerStateEraSummaries)?;
	match eras_summaries {
		OgmiosResponse::QueryLedgerStateEraSummaries(eras_summaries) => Ok(eras_summaries),
		other => Err(anyhow::anyhow!(format!(
			"Unexpected response from Ogmios when quering for era summaries: {other:?}"
		))),
	}
}

pub(crate) fn get_shelley_config<C: IOContext>(
	addr: &str,
	context: &C,
) -> anyhow::Result<ShelleyGenesisConfiguration> {
	let response = context.ogmios_rpc(addr, OgmiosRequest::QueryNetworkShelleyGenesis)?;
	match response {
        OgmiosResponse::QueryNetworkShelleyGenesis(shelley_config) => Ok(shelley_config),
        other => Err(anyhow::anyhow!(format!("Unexpected response from Ogmios when quering for shelley genesis configuration: {other:?}"))),
    }
}

fn caradano_parameters(
	eras_summaries: Vec<EraSummary>,
	shelley_config: ShelleyGenesisConfiguration,
) -> anyhow::Result<CardanoParameters> {
	let first_epoch_era = get_first_epoch_era(eras_summaries)?;
	Ok(CardanoParameters {
		security_parameter: shelley_config.security_parameter.into(),
		active_slots_coeff: shelley_config.active_slots_coefficient,
		epoch_duration_millis: (shelley_config.epoch_length as u64)
			.checked_mul(shelley_config.slot_length_millis)
			.ok_or_else(|| anyhow::anyhow!("Epoch duration overflow"))?,
		first_epoch_number: first_epoch_era.start.epoch,
		first_slot_number: first_epoch_era.start.slot,
		first_epoch_timestamp_millis: shelley_config
			.start_time
			.checked_add(first_epoch_era.start.time_seconds)
			.and_then(|seconds| seconds.checked_mul(1000))
			.ok_or_else(|| anyhow::anyhow!("First epoch timestamp overflow"))?,
		// This is a bug, we should not use network magic here
		network: CardanoNetwork(shelley_config.network_magic),
	})
}

// Partner Chains Main Chain follower supports only eras with 1 second slots.
// This functions gets the first era with 1 second slots,
// such that all following eras have the same slot length and epoch length.
fn get_first_epoch_era(eras_summaries: Vec<EraSummary>) -> Result<EraSummary, anyhow::Error> {
	let latest_era_parameters = eras_summaries
		.last()
		.ok_or_else(|| anyhow::anyhow!("No eras found"))?
		.parameters
		.clone();
	if latest_era_parameters.slot_length_millis != 1000 {
		return Err(anyhow::anyhow!(
			"Unexpected slot length in latest era, Partner Chains support only 1 second slots"
		));
	}
	let first_epoch_era = eras_summaries
		.into_iter()
		.find(|era| era.parameters == latest_era_parameters)
		.ok_or_else(|| anyhow::anyhow!("No eras found"))?;
	Ok(first_epoch_era)
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::config::{NetworkProtocol, CHAIN_CONFIG_FILE_PATH};
	use crate::ogmios::{EpochBoundary, EpochParameters, EraSummary};
	use crate::prepare_configuration::prepare_cardano_params::prepare_cardano_params;
	use crate::tests::{MockIO, MockIOContext};

	pub(crate) const PREPROD_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
		security_parameter: 2160,
		active_slots_coeff: 0.05,
		first_epoch_number: 4,
		first_slot_number: 86400,
		epoch_duration_millis: 432000000,
		first_epoch_timestamp_millis: 1655769600000,
		network: CardanoNetwork(1),
	};

	pub(crate) const PREVIEW_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
		security_parameter: 432,
		active_slots_coeff: 0.05,
		first_epoch_number: 0,
		first_slot_number: 0,
		epoch_duration_millis: 86400000,
		first_epoch_timestamp_millis: 1666656000000,
		network: CardanoNetwork(2),
	};

	#[test]
	fn should_persist_correct_cardano_params_for_preview() {
		test_saving_cardano_params_for_known_network(
			preview_eras_summaries(),
			preview_shelley_config(),
			PREVIEW_CARDANO_PARAMS,
		)
	}

	#[test]
	fn should_persist_correct_cardano_params_for_preprod() {
		test_saving_cardano_params_for_known_network(
			preprod_eras_summaries(),
			preprod_shelley_config(),
			PREPROD_CARDANO_PARAMS,
		)
	}

	fn test_saving_cardano_params_for_known_network(
		eras_summaries: Vec<EraSummary>,
		shelley_config: ShelleyGenesisConfiguration,
		expected_cardano_parameters: CardanoParameters,
	) {
		let ogmios_config = ServiceConfig {
			protocol: NetworkProtocol::Https,
			hostname: "ogmios.com".to_string(),
			port: 7654,
		};
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_FILE_PATH, serde_json::json!({}))
			.with_expected_io(vec![
				MockIO::ogmios_request(
					"https://ogmios.com:7654",
					OgmiosRequest::QueryLedgerStateEraSummaries,
					Ok(OgmiosResponse::QueryLedgerStateEraSummaries(eras_summaries)),
				),
				MockIO::ogmios_request(
					"https://ogmios.com:7654",
					OgmiosRequest::QueryNetworkShelleyGenesis,
					Ok(OgmiosResponse::QueryNetworkShelleyGenesis(shelley_config)),
				),
			]);
		let result = prepare_cardano_params(&ogmios_config, &mock_context);
		let params = result.expect("should succeed");
		assert_eq!(params, expected_cardano_parameters);
	}

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
			network_magic: 1,
			network: CardanoNetwork(1),
			security_parameter: 2160,
			active_slots_coefficient: 0.05,
			epoch_length: 432000,
			slot_length_millis: 1000,
			start_time: 1654041600,
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
			network_magic: 2,
			network: CardanoNetwork(1),
			security_parameter: 432,
			active_slots_coefficient: 0.05,
			epoch_length: 86400,
			slot_length_millis: 1000,
			start_time: 1666656000,
		}
	}
}
