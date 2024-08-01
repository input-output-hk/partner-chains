use crate::config::SidechainParams;
use crate::sidechain_main_cli_resources::SidechainMainCliResources;
use anyhow::anyhow;

pub fn sidechain_params_arguments(sidechain_params: &SidechainParams) -> String {
	format!("--sidechain-id {} --genesis-committee-hash-utxo {} --threshold-numerator {} --threshold-denominator {} --governance-authority {} --atms-kind plain-ecdsa-secp256k1",
			sidechain_params.chain_id,
			sidechain_params.genesis_committee_utxo,
			sidechain_params.threshold_numerator,
			sidechain_params.threshold_denominator,
			sidechain_params.governance_authority.to_hex_string())
}

pub fn runtime_config_arguments(
	runtime_config: &SidechainMainCliResources,
	payment_signing_key_path: &str,
) -> String {
	format!("--kupo-host {} --kupo-port {} {} --ogmios-host {} --ogmios-port {} {} --payment-signing-key-file {}",
			runtime_config.kupo.hostname,
			runtime_config.kupo.port,
			if runtime_config.kupo.protocol.is_secure() { "--kupo-secure" } else { "" },
			runtime_config.ogmios.hostname,
			runtime_config.ogmios.port,
			if runtime_config.ogmios.protocol.is_secure() { "--ogmios-secure" } else { "" },
			payment_signing_key_path
	)
}

pub fn check_for_kupo_ogmios_connection_error(
	response: &str,
	sidechain_main_cli_resources: &SidechainMainCliResources,
) -> anyhow::Result<()> {
	if response.contains("ECONNREFUSED") {
		let kupo_port_str = &sidechain_main_cli_resources.kupo.port.to_string();
		let (target, connection) = if response.contains(kupo_port_str) {
			("Kupo", sidechain_main_cli_resources.kupo.clone())
		} else {
			("Ogmios", sidechain_main_cli_resources.ogmios.clone())
		};
		Err(anyhow!(
			"Failed to connect to {target} at {connection}. Please check connection configuration and try again."
		))
	} else {
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	mod check_for_kupo_ogmios_connection_error {
		use crate::{
			config::config_fields::{KUPO_PORT, OGMIOS_PORT},
			sidechain_main_cli_resources::SidechainMainCliResources,
			smart_contracts::check_for_kupo_ogmios_connection_error,
		};

		#[test]
		fn should_display_user_friendly_message_when_kupo_connection_fails() {
			let error_string = format!("(UnknownContractError \"An error occurred when running CTL base monad: connect ECONNREFUSED ::1:{}\")", KUPO_PORT.default.unwrap());
			let sidechain_main_cli_resources = &SidechainMainCliResources::default();

			let result = check_for_kupo_ogmios_connection_error(
				&error_string,
				&sidechain_main_cli_resources,
			);

			assert_eq!(
				result.unwrap_err().to_string(),
				"Failed to connect to Kupo at http://localhost:1442. Please check connection configuration and try again."
			);
		}

		#[test]
		fn should_display_user_friendly_message_when_ogmios_connection_fails() {
			let error_string = format!("(UnknownContractError \"An error occurred when running CTL base monad: connect ECONNREFUSED ::1:{}\")", OGMIOS_PORT.default.unwrap());
			let sidechain_main_cli_resources = &SidechainMainCliResources::default();

			let result = check_for_kupo_ogmios_connection_error(
				&error_string,
				&sidechain_main_cli_resources,
			);

			assert_eq!(
				result.unwrap_err().to_string(),
				"Failed to connect to Ogmios at http://localhost:1337. Please check connection configuration and try again."
			);
		}
	}
}
