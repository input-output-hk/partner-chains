use crate::config::config_fields::{
	NODE_P2P_PORT, POSTGRES_CONNECTION_STRING, SIDECHAIN_BLOCK_BENEFICIARY,
};
use crate::config::{CardanoParameters, CHAIN_CONFIG_FILE_PATH, CHAIN_SPEC_PATH};
use crate::generate_keys::network_key_path;
use crate::io::IOContext;
use crate::keystore::*;
use crate::{config::config_fields, *};
use anyhow::anyhow;
use secp256k1::PublicKey;
use serde::Deserialize;
use sp_core::crypto::AccountId32;
use sp_runtime::app_crypto::ecdsa;
use sp_runtime::traits::IdentifyAccount;
use sp_runtime::MultiSigner;
use std::str::FromStr;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, clap::Parser)]
pub struct StartNodeCmd {
	#[arg(long)]
	silent: bool,
}

pub struct StartNodeConfig {
	pub substrate_node_base_path: String,
}

impl StartNodeConfig {
	pub fn load<C: IOContext>(context: &C) -> Self {
		Self {
			substrate_node_base_path: config_fields::SUBSTRATE_NODE_DATA_BASE_PATH
				.load_or_prompt_and_save(context),
		}
	}
	pub fn keystore_path(&self) -> String {
		keystore_path(&self.substrate_node_base_path)
	}
}

#[derive(Deserialize)]
pub struct StartNodeChainConfig {
	pub cardano: CardanoParameters,
	pub bootnodes: Vec<String>,
}

impl CmdRun for StartNodeCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let config = StartNodeConfig::load(context);

		if !check_keystore(&config, context)? || !check_chain_spec(context) {
			return Ok(());
		}

		let db_connection_string = POSTGRES_CONNECTION_STRING.load_or_prompt_and_save(context);

		let Some(chain_config) = load_chain_config(context)? else { return Ok(()) };

		let beneficiary = SIDECHAIN_BLOCK_BENEFICIARY
			.save_if_empty(block_beneficiary_from_cross_chain_key(&config, context)?, context);
		if !self.silent
			&& !prompt_values_fine(
				&config,
				&chain_config,
				&db_connection_string,
				&beneficiary,
				context,
			) {
			context.eprint("Aborting. Edit configuration files and rerun the command.");
			return Ok(());
		}

		start_node(config, chain_config, &db_connection_string, beneficiary, context)?;

		Ok(())
	}
}

fn load_chain_config<C: IOContext>(context: &C) -> anyhow::Result<Option<StartNodeChainConfig>> {
	let Some(chain_config_file) = context.read_file(CHAIN_CONFIG_FILE_PATH) else {
		context.eprint(&format!("⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} does not exists. Run prepare-configuration wizard first."));
		return Ok(None);
	};
	let chain_config = match serde_json::from_str::<StartNodeChainConfig>(&chain_config_file) {
		Ok(chain_config) => chain_config,
		Err(err) => {
			context.eprint(&format!("⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} is invalid: {err}. Run prepare-configuration wizard or fix errors manually."));
			return Ok(None);
		},
	};
	Ok(Some(chain_config))
}

#[rustfmt::skip]
fn prompt_values_fine<C: IOContext>(
	StartNodeConfig { substrate_node_base_path }: &StartNodeConfig,
	StartNodeChainConfig {
		cardano,
		bootnodes,
	}: &StartNodeChainConfig,
	db_connection_string: &str,
	beneficiary: &str,
	context: &C,
) -> bool
{
	context.eprint("The following values will be used to run the node:");
	context.eprint(&format!("    base path  = {}", substrate_node_base_path));
	context.eprint(&format!("    chain spec = {}", CHAIN_SPEC_PATH));
	context.eprint(&format!("    bootnodes  = [{}]", bootnodes.join(", ")));
	context.eprint("    environment:");
	context.eprint(&format!("        BLOCK_STABILITY_MARGIN             = {}", 0));
	context.eprint(&format!("        CARDANO_SECURITY_PARAMETER         = {}", cardano.security_parameter));
	context.eprint(&format!("        CARDANO_ACTIVE_SLOTS_COEFF         = {}", cardano.active_slots_coeff));
	context.eprint(&format!("        FIRST_EPOCH_TIMESTAMP_MILLIS       = {}", cardano.first_epoch_timestamp_millis));
	context.eprint(&format!("        EPOCH_DURATION_MILLIS              = {}", cardano.epoch_duration_millis));
	context.eprint(&format!("        FIRST_EPOCH_NUMBER                 = {}", cardano.first_epoch_number));
	context.eprint(&format!("        FIRST_SLOT_NUMBER                  = {}", cardano.first_slot_number));
	context.eprint(&format!("        DB_SYNC_POSTGRES_CONNECTION_STRING = {}", db_connection_string));
	context.eprint(&format!("        SIDECHAIN_BLOCK_BENEFICIARY        = {}", beneficiary));
	context.prompt_yes_no("Proceed?", true)
}

fn check_chain_spec<C: IOContext>(context: &C) -> bool {
	if context.file_exists(CHAIN_SPEC_PATH) {
		true
	} else {
		context.eprint(&format!("Chain spec file {} missing.", CHAIN_SPEC_PATH));
		context.eprint("Please run the create-chain-spec wizard first or you can get it from your chain governance.");
		false
	}
}

fn check_keystore<C: IOContext>(config: &StartNodeConfig, context: &C) -> anyhow::Result<bool> {
	let existing_keys = context.list_directory(&config.keystore_path())?.unwrap_or_default();
	Ok(key_present(&AURA, &existing_keys, context)
		&& key_present(&GRANDPA, &existing_keys, context)
		&& key_present(&CROSS_CHAIN, &existing_keys, context))
}

fn key_present<C: IOContext>(key: &KeyDefinition, existing_keys: &[String], context: &C) -> bool {
	if find_existing_key(existing_keys, key).is_none() {
		context.eprint(&format!(
			"⚠️ {} key is missing from the keystore. Please run generate-keys wizard first.",
			key.name
		));
		false
	} else {
		true
	}
}

fn block_beneficiary_from_cross_chain_key(
	config: &StartNodeConfig,
	context: &impl IOContext,
) -> anyhow::Result<String> {
	let existing_keys = context.list_directory(&config.keystore_path())?.unwrap_or_default();
	let key = find_existing_key(&existing_keys, &CROSS_CHAIN).ok_or(anyhow!(
		"⚠️ {} key is missing from the keystore. Please run generate-keys wizard first.",
		CROSS_CHAIN.name
	))?;
	account_id_hex_from_ecdsa_key(&key)
}

fn account_id_hex_from_ecdsa_key(key: &str) -> anyhow::Result<String> {
	let trimmed = key.trim_start_matches("0x");
	let pk = PublicKey::from_str(trimmed)?;
	let account_id: AccountId32 = MultiSigner::from(ecdsa::Public::from(pk)).into_account();
	Ok(hex::encode(account_id))
}

pub fn start_node<C: IOContext>(
	StartNodeConfig { substrate_node_base_path }: StartNodeConfig,
	StartNodeChainConfig {
		cardano:
			CardanoParameters {
				security_parameter,
				active_slots_coeff,
				first_epoch_number,
				first_slot_number,
				epoch_duration_millis,
				first_epoch_timestamp_millis,
				slot_duration_millis,
			},
		bootnodes,
	}: StartNodeChainConfig,
	db_connection_string: &str,
	beneficiary: String,
	context: &C,
) -> anyhow::Result<()> {
	let executable = context.current_executable()?;
	let environment_prefix = format!(
		"CARDANO_SECURITY_PARAMETER='{security_parameter}' \\
         CARDANO_ACTIVE_SLOTS_COEFF='{active_slots_coeff}' \\
         DB_SYNC_POSTGRES_CONNECTION_STRING='{db_connection_string}' \\
         MC__FIRST_EPOCH_TIMESTAMP_MILLIS='{first_epoch_timestamp_millis}' \\
         MC__EPOCH_DURATION_MILLIS='{epoch_duration_millis}' \\
         MC__SLOT_DURATION_MILLIS='{slot_duration_millis}' \\
         MC__FIRST_EPOCH_NUMBER='{first_epoch_number}' \\
         MC__FIRST_SLOT_NUMBER='{first_slot_number}' \\
         BLOCK_STABILITY_MARGIN='0' \\
		 SIDECHAIN_BLOCK_BENEFICIARY='{beneficiary}' \\
"
	);
	let bootnodes = bootnodes
		.iter()
		.map(|bootnode| format!("--bootnodes {}", bootnode))
		.collect::<Vec<String>>()
		.join(" ");

	let ws_port = NODE_P2P_PORT.save_if_empty(
		NODE_P2P_PORT
			.default
			.expect("Default NODE_WS_PORT should always be set")
			.parse()
			.expect("Default NODE_WS_PORT should be valid u16"),
		context,
	);
	let keystore_path = keystore_path(&substrate_node_base_path);
	let network_key_path = network_key_path(&substrate_node_base_path);
	context.run_command(&format!(
		"{environment_prefix} {executable} --validator --chain {CHAIN_SPEC_PATH} --base-path {substrate_node_base_path} --keystore-path {keystore_path} --node-key-file {network_key_path} --port {ws_port} {bootnodes}",
	))?;

	Ok(())
}
