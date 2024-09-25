use self::config::KEYS_FILE_PATH;
use crate::config::config_values::DEFAULT_CHAIN_NAME;
use crate::io::IOContext;
use crate::keystore::*;
use crate::permissioned_candidates::PermissionedCandidateKeys;
use crate::{config::config_fields, *};
use anyhow::{anyhow, Context};
use serde::Deserialize;
use sp_core::{ed25519, Pair};

#[cfg(test)]
mod tests;

#[derive(Debug, clap::Parser)]
pub struct GenerateKeysCmd {}

#[derive(Debug)]
pub struct GenerateKeysConfig {
	pub chain_name: String,
	pub substrate_node_base_path: String,
	pub node_executable: String,
}
impl GenerateKeysConfig {
	pub fn load<C: IOContext>(context: &C) -> Self {
		// ETCM-7825: hardcoded node executable
		let node_executable = config_fields::NODE_EXECUTABLE
			.save_if_empty(config_fields::NODE_EXECUTABLE_DEFAULT.to_string(), context);
		Self {
			chain_name: DEFAULT_CHAIN_NAME.into(),
			substrate_node_base_path: config_fields::SUBSTRATE_NODE_DATA_BASE_PATH
				.load_or_prompt_and_save(context),
			node_executable,
		}
	}
	fn keystore_path(&self) -> String {
		keystore_path(&self.substrate_node_base_path, &self.chain_name)
	}
	fn network_key_path(&self) -> String {
		let Self { chain_name, substrate_node_base_path, .. } = self;
		network_key_path(substrate_node_base_path, chain_name)
	}
}

pub fn network_key_path(substrate_node_base_path: &str, chain_name: &str) -> String {
	format!("{substrate_node_base_path}/chains/{chain_name}/network/secret_ed25519")
}
pub fn keystore_path(substrate_node_base_path: &str, chain_name: &str) -> String {
	format!("{substrate_node_base_path}/chains/{chain_name}/keystore")
}

impl CmdRun for GenerateKeysCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.eprint(
			"This 🧙 wizard will generate the following keys and save them to your node's keystore:"
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

pub fn verify_executable<C: IOContext>(
	GenerateKeysConfig { node_executable, .. }: &GenerateKeysConfig,
	context: &C,
) -> anyhow::Result<()> {
	if !context.file_exists(node_executable) {
		return Err(anyhow!("Partner Chains Node executable file ({node_executable}) is missing"));
	}
	Ok(())
}

pub fn set_dummy_env_vars<C: IOContext>(context: &C) {
	context.set_env_var("CHAIN_ID", "0");
	context.set_env_var("THRESHOLD_NUMERATOR", "0");
	context.set_env_var("THRESHOLD_DENOMINATOR", "0");
	context.set_env_var(
		"GENESIS_COMMITTEE_UTXO",
		"0000000000000000000000000000000000000000000000000000000000000000#0",
	);
	context.set_env_var(
		"GOVERNANCE_AUTHORITY",
		"00000000000000000000000000000000000000000000000000000000",
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

pub fn generate_spo_keys<C: IOContext>(
	config: &GenerateKeysConfig,
	context: &C,
) -> anyhow::Result<()> {
	if prompt_can_write("keys file", KEYS_FILE_PATH, context) {
		let cross_chain_key = generate_or_load_key(config, context, &CROSS_CHAIN)?;
		context.enewline();
		let grandpa_key = generate_or_load_key(config, context, &GRANDPA)?;
		context.enewline();
		let aura_key = generate_or_load_key(config, context, &AURA)?;
		context.enewline();

		let public_keys_json = serde_json::to_string_pretty(&PermissionedCandidateKeys {
			sidechain_pub_key: cross_chain_key,
			aura_pub_key: aura_key,
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

pub fn generate_network_key<C: IOContext>(
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
	GenerateKeysConfig { substrate_node_base_path, node_executable, .. }: &GenerateKeysConfig,
	context: &C,
) -> anyhow::Result<()> {
	context.run_command(&format!(
		"{node_executable} key generate-node-key --base-path {substrate_node_base_path}"
	))?;
	Ok(())
}

pub fn decode_network_key(key_str: &str) -> anyhow::Result<ed25519::Pair> {
	hex::decode(key_str)
		.context("Invalid hex")
		.and_then(|slice| ed25519::Pair::from_seed_slice(&slice).context("Invalid ed25519 bytes"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyGenerationOutput {
	public_key: String,
	secret_phrase: String,
}

pub fn generate_keys<C: IOContext>(
	context: &C,
	executable: &str,
	KeyDefinition { scheme, name, .. }: &KeyDefinition,
) -> anyhow::Result<KeyGenerationOutput> {
	context.eprint(&format!("⚙️ Generating {name} ({scheme}) key"));
	let output = context
		.run_command(&format!("{executable} key generate --scheme {scheme} --output-type json"))?;

	serde_json::from_str(&output)
		.map_err(|_| anyhow!("Failed to parse generated keys json: {output}"))
}

pub fn store_keys<C: IOContext>(
	context: &C,
	GenerateKeysConfig { chain_name, substrate_node_base_path: base_path, node_executable }: &GenerateKeysConfig,
	key_def: &KeyDefinition,
	KeyGenerationOutput { secret_phrase, public_key }: &KeyGenerationOutput,
) -> anyhow::Result<()> {
	let KeyDefinition { scheme, key_type, name } = key_def;
	context.eprint(&format!("💾 Inserting {name} ({scheme}) key"));
	let cmd = format!("{node_executable} key insert --base-path {base_path} --scheme {scheme} --key-type {key_type} --suri '{secret_phrase}'");
	let _ = context.run_command(&cmd)?;
	let store_path =
		format!("{base_path}/chains/{chain_name}/keystore/{}{public_key}", key_def.key_type_hex(),);
	context.eprint(&format!("💾 {name} key stored at {store_path}",));
	Ok(())
}

pub fn generate_or_load_key<C: IOContext>(
	config: &GenerateKeysConfig,
	context: &C,
	key_def: &KeyDefinition,
) -> anyhow::Result<String> {
	let GenerateKeysConfig { node_executable, .. } = config;
	let keystore_path = config.keystore_path();
	let existing_keys = context.list_directory(&keystore_path)?.unwrap_or_default();

	if let Some(key) = find_existing_key(&existing_keys, key_def) {
		if context.prompt_yes_no(
			&format!("A {} key already exists in store: {key} - overwrite it?", key_def.name),
			false,
		) {
			let new_key = generate_keys(context, node_executable, key_def)?;
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
		let new_key = generate_keys(context, node_executable, key_def)?;
		store_keys(context, config, key_def, &new_key)?;

		Ok(new_key.public_key)
	}
}
