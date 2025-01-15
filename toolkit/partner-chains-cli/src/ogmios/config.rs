use crate::config::config_fields::*;
use crate::config::ServiceConfig;
use crate::io::IOContext;

pub(crate) const OGMIOS_REQUIRED: &str =
	"Partner Chains Smart Contracts require access to Ogmios. Please provide its configuration.";

pub(crate) fn establish_ogmios_configuration<C: IOContext>(
	context: &C,
) -> anyhow::Result<ServiceConfig> {
	context.print(OGMIOS_REQUIRED);
	prompt_ogmios_configuration(context)
}

pub(crate) fn prompt_ogmios_configuration<C: IOContext>(
	context: &C,
) -> anyhow::Result<ServiceConfig> {
	let ogmios_protocol = OGMIOS_PROTOCOL
		.select_options_with_default_from_file_and_save(OGMIOS_PROTOCOL.name, context)
		.map_err(anyhow::Error::msg)?;
	let ogmios_hostname = OGMIOS_HOSTNAME.prompt_with_default_from_file_and_save(context);
	let ogmios_port = OGMIOS_PORT.prompt_with_default_from_file_parse_and_save(context)?;
	Ok(ServiceConfig { protocol: ogmios_protocol, hostname: ogmios_hostname, port: ogmios_port })
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;
	use crate::config::NetworkProtocol;
	use crate::prepare_configuration::tests::{
		prompt_multi_option_with_default_and_save_to_existing_file,
		prompt_with_default_and_save_to_existing_file,
	};
	use crate::tests::MockIO;
	use std::str::FromStr;

	pub(crate) fn default_ogmios_service_config() -> ServiceConfig {
		ServiceConfig {
			protocol: OGMIOS_PROTOCOL
				.default
				.and_then(|p| NetworkProtocol::from_str(p).ok())
				.unwrap_or(NetworkProtocol::Http),
			hostname: OGMIOS_HOSTNAME.default.unwrap_or("localhost").to_string(),
			port: OGMIOS_PORT.default.unwrap_or("1337").parse().unwrap(),
		}
	}

	/// Assumption for this function is that resources config file exists, so tests context should have it.
	pub(crate) fn establish_ogmios_configuration_io(
		existing_config: Option<ServiceConfig>,
		config_to_set: ServiceConfig,
	) -> MockIO {
		let default_config = existing_config.unwrap_or(default_ogmios_service_config());
		MockIO::Group(vec![
			MockIO::print(OGMIOS_REQUIRED),
			prompt_ogmios_configuration_io(&default_config, &config_to_set),
		])
	}

	pub(crate) fn prompt_ogmios_configuration_io(
		default_config: &ServiceConfig,
		config_to_set: &ServiceConfig,
	) -> MockIO {
		MockIO::Group(vec![
			prompt_multi_option_with_default_and_save_to_existing_file(
				OGMIOS_PROTOCOL,
				Some(&default_config.protocol.to_string()),
				&config_to_set.protocol.to_string(),
			),
			prompt_with_default_and_save_to_existing_file(
				OGMIOS_HOSTNAME,
				Some(&default_config.hostname),
				&config_to_set.hostname,
			),
			prompt_with_default_and_save_to_existing_file(
				OGMIOS_PORT,
				Some(&default_config.port.to_string()),
				&config_to_set.port.to_string(),
			),
		])
	}
}
