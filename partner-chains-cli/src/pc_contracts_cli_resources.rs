use crate::config::config_fields::*;
use crate::config::{NetworkProtocol, ServiceConfig};
use crate::io::IOContext;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub(crate) struct PcContractsCliResources {
	pub(crate) kupo: ServiceConfig,
	pub(crate) ogmios: ServiceConfig,
}

impl Default for PcContractsCliResources {
	fn default() -> Self {
		Self {
			kupo: ServiceConfig {
				protocol: KUPO_PROTOCOL
					.default
					.and_then(|p| NetworkProtocol::from_str(p).ok())
					.unwrap_or(NetworkProtocol::Http),
				hostname: KUPO_HOSTNAME.default.unwrap_or("localhost").to_string(),
				port: KUPO_PORT.default.unwrap_or("1442").parse().unwrap(),
			},
			ogmios: ServiceConfig {
				protocol: OGMIOS_PROTOCOL
					.default
					.and_then(|p| NetworkProtocol::from_str(p).ok())
					.unwrap_or(NetworkProtocol::Http),
				hostname: OGMIOS_HOSTNAME.default.unwrap_or("localhost").to_string(),
				port: OGMIOS_PORT.default.unwrap_or("1337").parse().unwrap(),
			},
		}
	}
}

pub(crate) const KUPO_AND_OGMIOS_REQUIRED: &str = "Partner Chains Smart Contracts require access to Kupo and Ogmios. Please provide their configuration.";

pub(crate) fn establish_pc_contracts_cli_configuration<C: IOContext>(
	context: &C,
) -> anyhow::Result<PcContractsCliResources> {
	context.print(KUPO_AND_OGMIOS_REQUIRED);
	let kupo_protocol = KUPO_PROTOCOL
		.select_options_with_default_from_file_and_save(KUPO_PROTOCOL.name, context)
		.map_err(anyhow::Error::msg)?;
	let kupo_hostname = KUPO_HOSTNAME.prompt_with_default_from_file_and_save(context);
	let kupo_port = KUPO_PORT.prompt_with_default_from_file_parse_and_save(context)?;
	let ogmios_protocol = OGMIOS_PROTOCOL
		.select_options_with_default_from_file_and_save(OGMIOS_PROTOCOL.name, context)
		.map_err(anyhow::Error::msg)?;
	let ogmios_hostname = OGMIOS_HOSTNAME.prompt_with_default_from_file_and_save(context);
	let ogmios_port = OGMIOS_PORT.prompt_with_default_from_file_parse_and_save(context)?;
	Ok(PcContractsCliResources {
		kupo: ServiceConfig { protocol: kupo_protocol, hostname: kupo_hostname, port: kupo_port },
		ogmios: ServiceConfig {
			protocol: ogmios_protocol,
			hostname: ogmios_hostname,
			port: ogmios_port,
		},
	})
}

#[cfg(test)]
pub(crate) mod tests {
	use crate::prepare_configuration::tests::{
		prompt_multi_option_with_default_and_save_to_existing_file,
		prompt_with_default_and_save_to_existing_file,
	};
	use crate::pc_contracts_cli_resources::*;
	use crate::tests::MockIO;

	/// Assumption for this function is that resources config file exists, so tests context should have it.
	pub(crate) fn establish_pc_contracts_cli_configuration_io(
		existing_config: Option<PcContractsCliResources>,
		config_to_set: PcContractsCliResources,
	) -> MockIO {
		let default_config = existing_config.unwrap_or_default();
		MockIO::Group(vec![
			MockIO::print(KUPO_AND_OGMIOS_REQUIRED),
			prompt_multi_option_with_default_and_save_to_existing_file(
				KUPO_PROTOCOL,
				Some(&default_config.kupo.protocol.to_string()),
				&config_to_set.kupo.protocol.to_string(),
			),
			prompt_with_default_and_save_to_existing_file(
				KUPO_HOSTNAME,
				Some(&default_config.kupo.hostname),
				&config_to_set.kupo.hostname,
			),
			prompt_with_default_and_save_to_existing_file(
				KUPO_PORT,
				Some(&default_config.kupo.port.to_string()),
				&config_to_set.kupo.port.to_string(),
			),
			prompt_multi_option_with_default_and_save_to_existing_file(
				OGMIOS_PROTOCOL,
				Some(&default_config.ogmios.protocol.to_string()),
				&config_to_set.ogmios.protocol.to_string(),
			),
			prompt_with_default_and_save_to_existing_file(
				OGMIOS_HOSTNAME,
				Some(&default_config.ogmios.hostname),
				&config_to_set.ogmios.hostname,
			),
			prompt_with_default_and_save_to_existing_file(
				OGMIOS_PORT,
				Some(&default_config.ogmios.port.to_string()),
				&config_to_set.ogmios.port.to_string(),
			),
		])
	}
}
