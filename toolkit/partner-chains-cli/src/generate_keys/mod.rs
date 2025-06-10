use self::config::KEYS_FILE_PATH;
use crate::io::IOContext;
use crate::keystore::*;
use crate::permissioned_candidates::PermissionedCandidateKeys;
use crate::{config::config_fields, *};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use sp_core::{Pair, ed25519};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, clap::Parser)]
pub struct GenerateKeysCmd {}

#[derive(Debug)]
pub struct GenerateKeysConfig {
	pub substrate_node_base_path: String,
}
impl GenerateKeysConfig {
	pub(crate) fn load<C: IOContext>(context: &C) -> Self {
		Self {
			substrate_node_base_path: config_fields::SUBSTRATE_NODE_DATA_BASE_PATH
				.load_or_prompt_and_save(context),
		}
	}
	fn keystore_path(&self) -> String {
		keystore_path(&self.substrate_node_base_path)
	}

	fn network_key_path(&self) -> String {
		let Self { substrate_node_base_path, .. } = self;
		network_key_path(substrate_node_base_path)
	}
}

fn network_key_directory(substrate_node_base_path: &str) -> String {
	format!("{substrate_node_base_path}/network")
}

pub(crate) fn network_key_path(substrate_node_base_path: &str) -> String {
	format!("{}/secret_ed25519", network_key_directory(substrate_node_base_path))
}

impl CmdRun for GenerateKeysCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.eprint(
			"This 🧙 wizard will generate the following keys and save them to your node's keystore:",
		);
		context.eprint("→  an ECDSA Cross-chain key");
		context.eprint("→  an ED25519 Grandpa key");
		context.eprint("→  an SR25519 Aura key");
		context.eprint("It will also generate a network key for your node if needed.");
		context.enewline();

		set_dummy_env_vars(context);

		let config = GenerateKeysConfig::load(context);
		context.enewline();

		generate_spo_keys(&config, context)?;
		context.enewline();

		generate_network_key(&config, context)?;
		context.enewline();

		context.eprint("🚀 All done!");

		Ok(())
	}
}

fn set_dummy_env_vars<C: IOContext>(context: &C) {
	context.set_env_var(
		"GENESIS_UTXO",
		"0000000000000000000000000000000000000000000000000000000000000000#0",
	);
	context.set_env_var("COMMITTEE_CANDIDATE_ADDRESS", "addr_10000");
	context.set_env_var(
		"D_PARAMETER_POLICY_ID",
		"00000000000000000000000000000000000000000000000000000000",
	);
	context.set_env_var(
		"PERMISSIONED_CANDIDATES_POLICY_ID",
		"00000000000000000000000000000000000000000000000000000000",
	);
	context.set_env_var(
		"NATIVE_TOKEN_POLICY_ID",
		"00000000000000000000000000000000000000000000000000000000",
	);
	context.set_env_var(
		"NATIVE_TOKEN_ASSET_NAME",
		"00000000000000000000000000000000000000000000000000000000",
	);
	context.set_env_var(
		"ILLIQUID_SUPPLY_VALIDATOR_ADDRESS",
		"00000000000000000000000000000000000000000000000000000000",
	);
}

pub(crate) fn generate_spo_keys<C: IOContext>(
	config: &GenerateKeysConfig,
	context: &C,
) -> anyhow::Result<()> {
	if prompt_can_write("keys file", KEYS_FILE_PATH, context) {
		let cross_chain_key = generate_or_load_key(config, context, &CROSS_CHAIN)?;
		context.enewline();
		let beefy_key = generate_or_load_key(config, context, &BEEFY)?;
		context.enewline();
		let grandpa_key = generate_or_load_key(config, context, &GRANDPA)?;
		context.enewline();
		let aura_key = generate_or_load_key(config, context, &AURA)?;
		context.enewline();

		let public_keys_json = serde_json::to_string_pretty(&PermissionedCandidateKeys {
			sidechain_pub_key: cross_chain_key,
			aura_pub_key: aura_key,
			beefy_pub_key: beefy_key,
			grandpa_pub_key: grandpa_key,
		})
		.expect("Failed to serialize public keys");
		context.write_file(KEYS_FILE_PATH, &public_keys_json);

		context.eprint(&format!(
			"🔑 The following public keys were generated and saved to the {} file:",
			KEYS_FILE_PATH,
		));
		context.print(&(public_keys_json).to_string());
		context.eprint("You may share them with your chain governance authority");
		context.eprint("if you wish to be included as a permissioned candidate.");
	} else {
		context.eprint("Refusing to overwrite keys file - skipping");
	}
	Ok(())
}

pub(crate) fn generate_network_key<C: IOContext>(
	config: &GenerateKeysConfig,
	context: &C,
) -> anyhow::Result<()> {
	let maybe_existing_key =
		context.read_file(&config.network_key_path()).as_deref().map(decode_network_key);
	match maybe_existing_key {
		Some(Ok(_)) => {
			context.eprint(
				"🔑 A valid network key is already present in the keystore, skipping generation",
			);
		},
		None => {
			context.eprint("⚙️ Generating network key");
			run_generate_network_key(config, context)?;
		},
		Some(Err(err)) => {
			context.eprint(&format!(
				"⚠️ Network key in keystore is invalid ({}), wizard will regenerate it",
				err,
			));
			context.eprint("⚙️ Regenerating the network key");
			context.delete_file(&config.network_key_path())?;
			run_generate_network_key(config, context)?;
		},
	};
	Ok(())
}

fn run_generate_network_key<C: IOContext>(
	config: &GenerateKeysConfig,
	context: &C,
) -> anyhow::Result<()> {
	let node_executable = context.current_executable()?;
	let network_key_directory = network_key_directory(&config.substrate_node_base_path);
	let network_key_path = config.network_key_path();
	context.run_command(&format!("mkdir -p {network_key_directory}"))?;
	context.run_command(&format!(
		"{node_executable} key generate-node-key --file {network_key_path}"
	))?;
	Ok(())
}

fn decode_network_key(key_str: &str) -> anyhow::Result<ed25519::Pair> {
	hex::decode(key_str)
		.context("Invalid hex")
		.and_then(|slice| ed25519::Pair::from_seed_slice(&slice).context("Invalid ed25519 bytes"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyGenerationOutput {
	public_key: String,
	secret_phrase: String,
}

fn generate_keys<C: IOContext>(
	context: &C,
	KeyDefinition { scheme, name, .. }: &KeyDefinition,
) -> anyhow::Result<KeyGenerationOutput> {
	let executable = context.current_executable()?;
	context.eprint(&format!("⚙️ Generating {name} ({scheme}) key"));
	let output = context
		.run_command(&format!("{executable} key generate --scheme {scheme} --output-type json"))?;

	serde_json::from_str(&output)
		.map_err(|_| anyhow!("Failed to parse generated keys json: {output}"))
}

fn store_keys<C: IOContext>(
	context: &C,
	GenerateKeysConfig { substrate_node_base_path: base_path }: &GenerateKeysConfig,
	key_def: &KeyDefinition,
	KeyGenerationOutput { secret_phrase, public_key }: &KeyGenerationOutput,
) -> anyhow::Result<()> {
	let node_executable = context.current_executable()?;
	let KeyDefinition { scheme, key_type, name } = key_def;
	context.eprint(&format!("💾 Inserting {name} ({scheme}) key"));
	let keystore_path = keystore_path(base_path);
	let cmd = format!(
		"{node_executable} key insert --keystore-path {keystore_path} --scheme {scheme} --key-type {key_type} --suri '{secret_phrase}'"
	);
	let _ = context.run_command(&cmd)?;
	let store_path = format!("{}/{}{public_key}", keystore_path, key_def.key_type_hex(),);
	context.eprint(&format!("💾 {name} key stored at {store_path}",));
	Ok(())
}

fn generate_or_load_key<C: IOContext>(
	config: &GenerateKeysConfig,
	context: &C,
	key_def: &KeyDefinition,
) -> anyhow::Result<String> {
	let keystore_path = config.keystore_path();
	let existing_keys = context.list_directory(&keystore_path)?.unwrap_or_default();

	if let Some(key) = find_existing_key(&existing_keys, key_def) {
		if context.prompt_yes_no(
			&format!("A {} key already exists in store: {key} - overwrite it?", key_def.name),
			false,
		) {
			let new_key = generate_keys(context, key_def)?;
			store_keys(context, config, key_def, &new_key)?;

			let old_key_path = format!("{keystore_path}/{}{key}", key_def.key_type_hex());
			context
				.delete_file(&old_key_path)
				.context(format!("Failed to overwrite {} key at {old_key_path}", key_def.name))?;

			Ok(new_key.public_key)
		} else {
			Ok(format!("0x{key}"))
		}
	} else {
		let new_key = generate_keys(context, key_def)?;
		store_keys(context, config, key_def, &new_key)?;

		Ok(new_key.public_key)
	}
}
