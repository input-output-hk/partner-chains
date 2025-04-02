use crate::config::CardanoParameters;
use crate::io::IOContext;

pub fn set_data_sources_env(
	context: &impl IOContext,
	config: &CardanoParameters,
	postgres_connection_string: &str,
) {
	context.set_env_var("DB_SYNC_POSTGRES_CONNECTION_STRING", postgres_connection_string);
	context.set_env_var("CARDANO_SECURITY_PARAMETER", &config.security_parameter.to_string());
	context.set_env_var("CARDANO_ACTIVE_SLOTS_COEFF", &config.active_slots_coeff.to_string());
	context.set_env_var("BLOCK_STABILITY_MARGIN", "0");
	context.set_env_var(
		"MC__FIRST_EPOCH_TIMESTAMP_MILLIS",
		&config.first_epoch_timestamp_millis.to_string(),
	);
	context.set_env_var("MC__FIRST_EPOCH_NUMBER", &config.first_epoch_number.to_string());
	context.set_env_var("MC__EPOCH_DURATION_MILLIS", &config.epoch_duration_millis.to_string());
	context.set_env_var("MC__FIRST_SLOT_NUMBER", &config.first_slot_number.to_string());
}
