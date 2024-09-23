use crate::config::config_fields::{
	CARDANO_ACTIVE_SLOTS_COEFF, CARDANO_EPOCH_DURATION_MILLIS, CARDANO_FIRST_EPOCH_NUMBER,
	CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS, CARDANO_FIRST_SLOT_NUMBER, CARDANO_SECURITY_PARAMETER,
};
use crate::config::CardanoParameters;
use crate::io::IOContext;

pub(crate) const PREPROD_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
	security_parameter: 2160,
	active_slots_coeff: 0.05,
	first_epoch_number: 4,
	first_slot_number: 86400,
	epoch_duration_millis: 432000000,
	first_epoch_timestamp_millis: 1655769600000,
};

pub(crate) const MAINNET_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
	security_parameter: 2160,
	active_slots_coeff: 0.05,
	first_epoch_number: 208,
	first_slot_number: 4492800,
	epoch_duration_millis: 432000000,
	first_epoch_timestamp_millis: 1596059091000,
};

pub(crate) const PREVIEW_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
	security_parameter: 432,
	active_slots_coeff: 0.05,
	first_epoch_number: 0,
	first_slot_number: 0,
	epoch_duration_millis: 86400000,
	first_epoch_timestamp_millis: 1666656000000,
};

pub fn prepare_cardano_params<C: IOContext>(
	context: &C,
	cardano_network: u32,
) -> anyhow::Result<CardanoParameters> {
	Ok(match cardano_network {
		0 => {
			MAINNET_CARDANO_PARAMS.save(context);
			MAINNET_CARDANO_PARAMS
		},
		1 => {
			PREPROD_CARDANO_PARAMS.save(context);
			PREPROD_CARDANO_PARAMS
		},
		2 => {
			PREVIEW_CARDANO_PARAMS.save(context);
			PREVIEW_CARDANO_PARAMS
		},
		_ => prompt_for_custom_cardano_params(context)?,
	})
}

fn prompt_for_custom_cardano_params(context: &impl IOContext) -> anyhow::Result<CardanoParameters> {
	Ok(CardanoParameters {
		security_parameter: CARDANO_SECURITY_PARAMETER
			.prompt_with_default_from_file_parse_and_save(context)?,
		active_slots_coeff: CARDANO_ACTIVE_SLOTS_COEFF
			.prompt_with_default_from_file_parse_and_save(context)?,
		first_epoch_number: CARDANO_FIRST_EPOCH_NUMBER
			.prompt_with_default_from_file_parse_and_save(context)?,
		first_slot_number: CARDANO_FIRST_SLOT_NUMBER
			.prompt_with_default_from_file_parse_and_save(context)?,
		epoch_duration_millis: CARDANO_EPOCH_DURATION_MILLIS
			.prompt_with_default_from_file_parse_and_save(context)?,
		first_epoch_timestamp_millis: CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS
			.prompt_with_default_from_file_parse_and_save(context)?,
	})
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::config::config_fields::CARDANO_SECURITY_PARAMETER;
	use crate::prepare_configuration::prepare_cardano_params::prepare_cardano_params;
	use crate::prepare_configuration::prepare_cardano_params::tests::scenarios::save_cardano_params;
	use crate::tests::{should_have_no_io_left, MockIOContext};
	use serde_json::Value;

	const CUSTOM_CARDANO_PARAMS: CardanoParameters = CardanoParameters {
		security_parameter: 431,
		active_slots_coeff: 0.07,
		first_epoch_number: 0,
		first_slot_number: 0,
		epoch_duration_millis: 86800000,
		first_epoch_timestamp_millis: 1866656000000,
	};

	pub mod scenarios {
		use super::*;
		use crate::config::config_fields::{
			CARDANO_ACTIVE_SLOTS_COEFF, CARDANO_EPOCH_DURATION_MILLIS, CARDANO_FIRST_EPOCH_NUMBER,
			CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS, CARDANO_FIRST_SLOT_NUMBER,
			CARDANO_SECURITY_PARAMETER,
		};
		use crate::prepare_configuration::tests::{
			prompt_with_default_and_save_to_existing_file, save_to_existing_file,
		};
		use crate::tests::MockIO;

		pub fn save_cardano_params(cardano_parameters: CardanoParameters) -> MockIO {
			MockIO::Group(vec![
				save_cardano_params_but_last(cardano_parameters.clone()),
				MockIO::file_read(CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS.config_file),
				MockIO::file_write_json(
					CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS.config_file,
					test_chain_config(cardano_parameters),
				),
			])
		}

		pub fn save_cardano_params_but_last(cardano_parameters: CardanoParameters) -> MockIO {
			MockIO::Group(vec![
				save_to_existing_file(
					CARDANO_SECURITY_PARAMETER,
					&cardano_parameters.security_parameter.to_string(),
				),
				save_to_existing_file(
					CARDANO_ACTIVE_SLOTS_COEFF,
					&cardano_parameters.active_slots_coeff.to_string(),
				),
				save_to_existing_file(
					CARDANO_FIRST_EPOCH_NUMBER,
					&cardano_parameters.first_epoch_number.to_string(),
				),
				save_to_existing_file(
					CARDANO_FIRST_SLOT_NUMBER,
					&cardano_parameters.first_slot_number.to_string(),
				),
				save_to_existing_file(
					CARDANO_EPOCH_DURATION_MILLIS,
					&cardano_parameters.epoch_duration_millis.to_string(),
				),
			])
		}

		pub fn prompt_for_custom_cardano_params() -> MockIO {
			MockIO::Group(vec![
				prompt_with_default_and_save_to_existing_file(
					CARDANO_SECURITY_PARAMETER,
					CARDANO_SECURITY_PARAMETER.default,
					&CUSTOM_CARDANO_PARAMS.security_parameter.to_string(),
				),
				prompt_with_default_and_save_to_existing_file(
					CARDANO_ACTIVE_SLOTS_COEFF,
					CARDANO_ACTIVE_SLOTS_COEFF.default,
					&CUSTOM_CARDANO_PARAMS.active_slots_coeff.to_string(),
				),
				prompt_with_default_and_save_to_existing_file(
					CARDANO_FIRST_EPOCH_NUMBER,
					CARDANO_FIRST_EPOCH_NUMBER.default,
					&CUSTOM_CARDANO_PARAMS.first_epoch_number.to_string(),
				),
				prompt_with_default_and_save_to_existing_file(
					CARDANO_FIRST_SLOT_NUMBER,
					CARDANO_FIRST_SLOT_NUMBER.default,
					&CUSTOM_CARDANO_PARAMS.first_slot_number.to_string(),
				),
				prompt_with_default_and_save_to_existing_file(
					CARDANO_EPOCH_DURATION_MILLIS,
					CARDANO_EPOCH_DURATION_MILLIS.default,
					&CUSTOM_CARDANO_PARAMS.epoch_duration_millis.to_string(),
				),
				prompt_with_default_and_save_to_existing_file(
					CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS,
					CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS.default,
					&CUSTOM_CARDANO_PARAMS.first_epoch_timestamp_millis.to_string(),
				),
			])
		}
	}

	#[test]
	fn should_persist_correct_cardano_params_for_preview() {
		test_saving_cardano_params_for_hardcoded_networks(PREVIEW_CARDANO_PARAMS, 2)
	}

	#[test]
	fn should_persist_correct_cardano_params_for_mainnet() {
		test_saving_cardano_params_for_hardcoded_networks(MAINNET_CARDANO_PARAMS, 0)
	}

	#[test]
	fn should_persist_correct_cardano_params_for_preprod() {
		test_saving_cardano_params_for_hardcoded_networks(PREPROD_CARDANO_PARAMS, 1)
	}

	#[test]
	fn prompt_for_custom_params() {
		let mock_context = MockIOContext::new()
			.with_json_file(CARDANO_SECURITY_PARAMETER.config_file, serde_json::json!({}))
			.with_expected_io(vec![scenarios::prompt_for_custom_cardano_params()]);
		let params = prepare_cardano_params(&mock_context, 3).unwrap();
		assert_eq!(params, CUSTOM_CARDANO_PARAMS)
	}

	fn test_saving_cardano_params_for_hardcoded_networks(
		cardano_parameters: CardanoParameters,
		cardano_network: u32,
	) {
		let mock_context = MockIOContext::new()
			.with_json_file(CARDANO_SECURITY_PARAMETER.config_file, serde_json::json!({}))
			.with_expected_io(vec![save_cardano_params(cardano_parameters.clone())]);
		let result = prepare_cardano_params(&mock_context, cardano_network);
		let params = result.expect("Expected the result to be a success");
		assert_eq!(params, cardano_parameters);
		should_have_no_io_left!(mock_context);
	}

	fn test_chain_config(cardano_parameters: CardanoParameters) -> Value {
		serde_json::json!({
			"cardano": {
				"security_parameter": cardano_parameters.security_parameter,
				"active_slots_coeff": cardano_parameters.active_slots_coeff,
				"first_epoch_number": cardano_parameters.first_epoch_number,
				"first_slot_number": cardano_parameters.first_slot_number,
				"epoch_duration_millis": cardano_parameters.epoch_duration_millis,
				"first_epoch_timestamp_millis": cardano_parameters.first_epoch_timestamp_millis
			},
		})
	}
}
