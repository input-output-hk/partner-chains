#[cfg(test)]
mod tests;

use crate::cardano_key::{get_mc_pkey_from_file, get_mc_pubkey_from_file};
use crate::config::config_fields::{
	CARDANO_COLD_VERIFICATION_KEY_FILE, CARDANO_PAYMENT_SIGNING_KEY_FILE,
};
use crate::config::CHAIN_CONFIG_FILE_PATH;
use crate::io::IOContext;
use crate::ogmios::config::establish_ogmios_configuration;
use crate::CmdRun;
use anyhow::anyhow;
use partner_chains_cardano_offchain::register::Deregister;

#[derive(Debug, clap::Parser)]
pub struct DeregisterCmd;

impl CmdRun for DeregisterCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let chain_config = read_chain_config_file(context)?;
		context.print(
		  &format!("This wizard will remove the specified candidate from the committee candidates based on the following chain parameters:\n{}.\nCommittee Candidate Validator Address is '{}'\n",
          &chain_config.chain_parameters,
          &chain_config.cardano_addresses.committee_candidates_address)
    	);
		context.print("Payment signing key and cold verification key used for registration are required to deregister.");
		let payment_signing_key_path =
			CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let payment_signing_key = get_mc_pkey_from_file(&payment_signing_key_path, context)?;
		let cold_vkey_path =
			CARDANO_COLD_VERIFICATION_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let stake_ownership_pub_key = get_mc_pubkey_from_file(&cold_vkey_path, context)?;
		let ogmios_config = establish_ogmios_configuration(context)?;
		let offchain = context.offchain_impl(&ogmios_config)?;

		let runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		runtime
			.block_on(offchain.deregister(
				chain_config.chain_parameters.genesis_utxo,
				payment_signing_key,
				stake_ownership_pub_key,
			))
			.map_err(|e| anyhow::anyhow!("Candidate deregistration failed: {e:?}!"))?;

		Ok(())
	}
}

fn read_chain_config_file<C: IOContext>(
	context: &C,
) -> Result<crate::config::ChainConfig, anyhow::Error> {
	let chain_config = crate::config::load_chain_config(context);
	chain_config
		.map_err(|_|anyhow!(
			"Couldn't parse chain configuration file {}. The chain configuration file that was used for registration is required in the working directory.",
			CHAIN_CONFIG_FILE_PATH))
}
