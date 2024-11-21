#[cfg(test)]
mod tests;

use crate::config::config_fields::{
	CARDANO_COLD_VERIFICATION_KEY_FILE, CARDANO_PAYMENT_SIGNING_KEY_FILE,
};
use crate::config::{
	get_cardano_network_from_file, ChainConfig, CHAIN_CONFIG_FILE_PATH, PC_CONTRACTS_CLI_PATH,
};
use crate::io::IOContext;
use crate::pc_contracts_cli_resources::{
	establish_pc_contracts_cli_configuration, PcContractsCliResources,
};
use crate::{smart_contracts, CmdRun};
use anyhow::{anyhow, Context};
use sidechain_domain::NetworkType;

#[derive(Debug, clap::Parser)]
pub struct DeregisterCmd;

impl CmdRun for DeregisterCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let (chain_config, cardano_network) = read_chain_config_file(context)?;
		context.print(
		  &format!("This wizard will remove the specified candidate from the committee candidates based on the following chain parameters:\n{}.\nCommittee Candidate Validator Address is '{}'\n",
          &chain_config.chain_parameters,
          &chain_config.cardano_addresses.committee_candidates_address)
    	);
		let (payment_signing_key_path, cold_vkey) = get_cardano_keys(context)?;
		let pc_contracts_cli_resources = establish_pc_contracts_cli_configuration(context)?;

		let command = build_command(
			cardano_network,
			cold_vkey,
			chain_config,
			pc_contracts_cli_resources,
			payment_signing_key_path,
		);
		let output = context.run_command(&command).context("Deregistration failed")?;
		Ok(context.print(&format!("Deregistration successful: {}", output)))
	}
}

fn read_chain_config_file<C: IOContext>(
	context: &C,
) -> Result<(crate::config::ChainConfig, NetworkType), anyhow::Error> {
	let chain_config = crate::config::load_chain_config(context);
	let cardano_network = get_cardano_network_from_file(context);
	chain_config.and_then(|chain_config| cardano_network.map(|cardano_network| (chain_config, cardano_network)))
		    .map_err(|_|anyhow!(
		        "Couldn't parse chain configuration file {}. The chain configuration file that was used for registration is required in the working directory.",
		        CHAIN_CONFIG_FILE_PATH))
}

fn get_cardano_keys<C: IOContext>(context: &C) -> Result<(String, String), anyhow::Error> {
	context.print("Payment signing key and verification key of cold key used for registration are required to deregister.");
	let payment_signing_key_path =
		CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
	let _ = get_mainchain_key_hex(context, &payment_signing_key_path)?;
	let cold_vkey_path =
		CARDANO_COLD_VERIFICATION_KEY_FILE.prompt_with_default_from_file_and_save(context);
	let cold_vkey = get_mainchain_key_hex(context, &cold_vkey_path)?;
	Ok((payment_signing_key_path, cold_vkey))
}

fn get_mainchain_key_hex<C: IOContext>(
	context: &C,
	key_path: &str,
) -> Result<String, anyhow::Error> {
	let cold_key = context
		.read_file(key_path)
		.ok_or_else(|| anyhow::anyhow!("Unable to read Cardano key file: {}", key_path))?;
	let json = serde_json::from_str::<serde_json::Value>(&cold_key)
		.map_err(|e| anyhow!("{} is not a valid Cardano key file: {}", key_path, e))?;
	let key = &json
		.pointer("/cborHex")
		.ok_or(anyhow!("{} is not a valid Cardano key file", key_path))?
		.as_str()
		.ok_or(anyhow!("{} is not a valid Cardano key file", key_path))?;
	Ok(key[4..].to_string())
}

fn build_command(
	cardano_network: NetworkType,
	cold_vkey: String,
	chain_config: ChainConfig,
	pc_contracts_cli_resources: PcContractsCliResources,
	payment_signing_key_path: String,
) -> String {
	format!(
        "{PC_CONTRACTS_CLI_PATH} deregister --network {} --ada-based-staking --spo-public-key {} {} {}",
        cardano_network,
        cold_vkey,
        smart_contracts::sidechain_params_arguments(&chain_config.chain_parameters),
        smart_contracts::runtime_config_arguments(
            &pc_contracts_cli_resources,
            &payment_signing_key_path
        )
	)
}
