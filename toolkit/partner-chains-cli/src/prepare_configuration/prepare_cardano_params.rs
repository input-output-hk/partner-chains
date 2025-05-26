use crate::config::{CardanoParameters, ServiceConfig};
use crate::io::IOContext;
use crate::ogmios::{
	EraSummary, OgmiosRequest, OgmiosResponse, ShelleyGenesisConfiguration, get_shelley_config,
};

pub fn prepare_cardano_params<C: IOContext>(
	ogmios_config: &ServiceConfig,
	context: &C,
) -> anyhow::Result<CardanoParameters> {
	let eras_summaries = get_eras_summaries(&ogmios_config, context)?;
	let shelley_config = get_shelley_config(&ogmios_config, context)?;
	caradano_parameters(eras_summaries, shelley_config)
}

fn get_eras_summaries<C: IOContext>(
	config: &ServiceConfig,
	context: &C,
) -> anyhow::Result<Vec<EraSummary>> {
	let eras_summaries = context.ogmios_rpc(config, OgmiosRequest::QueryLedgerStateEraSummaries)?;
	match eras_summaries {
		OgmiosResponse::QueryLedgerStateEraSummaries(eras_summaries) => Ok(eras_summaries),
		other => Err(anyhow::anyhow!(format!(
			"Unexpected response from Ogmios when quering for era summaries: {other:?}"
		))),
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
		slot_duration_millis: shelley_config.slot_length_millis,
	})
}

// This functions gets the first era
// such that all following eras have the same slot length and epoch length.
fn get_first_epoch_era(eras_summaries: Vec<EraSummary>) -> Result<EraSummary, anyhow::Error> {
	let latest_era = eras_summaries.last().ok_or_else(|| anyhow::anyhow!("No eras found"))?;
	let first_epoch_era = eras_summaries
		.iter()
		.find(|era| era.parameters == latest_era.parameters)
		.ok_or_else(|| anyhow::anyhow!("No eras found"))?;
	Ok(first_epoch_era.clone())
}

#[cfg(test)]
pub mod tests {

	use super::*;
	use crate::config::{CHAIN_CONFIG_FILE_PATH, NetworkProtocol};
	use crate::ogmios::EraSummary;
	use crate::ogmios::test_values::{
		preprod_eras_summaries, preprod_shelley_config, preview_eras_summaries,
		preview_shelley_config,
	};
	use crate::prepare_configuration::prepare_cardano_params::prepare_cardano_params;
	use crate::tests::{MockIO, MockIOContext};

	pub(crate) const PREPROD_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
		security_parameter: 2160,
		active_slots_coeff: 0.05,
		first_epoch_number: 4,
		first_slot_number: 86400,
		epoch_duration_millis: 432000000,
		first_epoch_timestamp_millis: 1655769600000,
		slot_duration_millis: 1000,
	};

	pub(crate) const PREVIEW_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
		security_parameter: 432,
		active_slots_coeff: 0.05,
		first_epoch_number: 0,
		first_slot_number: 0,
		epoch_duration_millis: 86400000,
		first_epoch_timestamp_millis: 1666656000000,
		slot_duration_millis: 1000,
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
			timeout_seconds: 180,
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
}
