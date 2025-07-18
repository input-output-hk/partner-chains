use self::config::KEYS_FILE_PATH;
use crate::io::IOContext;
use crate::keystore::*;
use crate::permissioned_candidates::PermissionedCandidateKeys;
use crate::{config::config_fields, *};
use anyhow::{Context, anyhow};
use serde::Deserialize;
use sidechain_domain::byte_string::ByteString;
use sp_core::{Pair, ed25519};
use std::collections::BTreeMap;
use std::marker::PhantomData;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, clap::Parser)]
pub struct GenerateKeysCmd<T: PartnerChainRuntime> {
	#[clap(skip)]
	_phantom: PhantomData<T>,
}

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

impl<T: PartnerChainRuntime> CmdRun for GenerateKeysCmd<T> {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.eprint(
			"This 🧙 wizard will generate the following keys and save them to your node's keystore:",
		);
		context.eprint(&format!("→ {} {} key", CROSS_CHAIN.scheme, CROSS_CHAIN.name));
		for key_def in T::key_definitions() {
			context.eprint(&format!("→ {} {} key", key_def.scheme, key_def.name));
		}
		context.eprint("It will also generate a network key for your node if needed.");
		context.enewline();

		let chain_spec_path = write_temp_chain_spec(
			context,
			T::create_chain_spec(&CreateChainSpecConfig::<T::Keys>::default()),
		);

		let config = GenerateKeysConfig::load(context);
		context.enewline();

		generate_spo_keys::<C, T>(&config, &chain_spec_path, context)?;

		context.enewline();

		generate_network_key(&config, &chain_spec_path, context)?;
		context.enewline();

		context.eprint("🚀 All done!");
		context.delete_file(&chain_spec_path)?;
		Ok(())
	}
}

fn write_temp_chain_spec<C: IOContext>(context: &C, chain_spec: serde_json::Value) -> String {
	let dir_path = context.new_tmp_dir();
	let dir_path = dir_path.to_str().expect("temp dir path is correct utf-8");
	let path = format!("{dir_path}/chain-spec.json");
	let content = format!("{chain_spec}");
	context.write_file(&path, &content);
	path
}

pub(crate) fn generate_spo_keys<C: IOContext, T: PartnerChainRuntime>(
	config: &GenerateKeysConfig,
	chain_spec_path: &str,
	context: &C,
) -> anyhow::Result<()> {
	if prompt_can_write("keys file", KEYS_FILE_PATH, context) {
		let partner_chains_key =
			generate_or_load_key(config, context, chain_spec_path, &CROSS_CHAIN)?;
		context.enewline();
		let mut keys: BTreeMap<String, ByteString> = BTreeMap::new();
		for key_definition in T::key_definitions() {
			let generated_key =
				generate_or_load_key(config, context, chain_spec_path, &key_definition)?;
			context.enewline();
			keys.insert(key_definition.key_type.to_owned(), generated_key);
		}

		let public_keys_json =
			serde_json::to_string_pretty(&PermissionedCandidateKeys { partner_chains_key, keys })
				.expect("PermissionedCandidateKeys have only UTF-8 encodable ids");
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
	chain_spec_path: &str,
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
			run_generate_network_key(config, chain_spec_path, context)?;
		},
		Some(Err(err)) => {
			context.eprint(&format!(
				"⚠️ Network key in keystore is invalid ({}), wizard will regenerate it",
				err,
			));
			context.eprint("⚙️ Regenerating the network key");
			context.delete_file(&config.network_key_path())?;
			run_generate_network_key(config, chain_spec_path, context)?;
		},
	};
	Ok(())
}

fn run_generate_network_key<C: IOContext>(
	config: &GenerateKeysConfig,
	chain_spec_path: &str,
	context: &C,
) -> anyhow::Result<()> {
	let node_executable = context.current_executable()?;
	let network_key_directory = network_key_directory(&config.substrate_node_base_path);
	let network_key_path = config.network_key_path();
	context.run_command(&format!("mkdir -p {network_key_directory}"))?;
	context.run_command(&format!(
		"{node_executable} key generate-node-key --chain {chain_spec_path} --file {network_key_path}"
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
	chain_spec_file_path: &str,
) -> anyhow::Result<()> {
	let node_executable = context.current_executable()?;
	let KeyDefinition { scheme, key_type, name } = key_def;
	context.eprint(&format!("💾 Inserting {name} ({scheme}) key"));
	let keystore_path = keystore_path(base_path);
	let cmd = format!(
		"{node_executable} key insert --chain {chain_spec_file_path} --keystore-path {keystore_path} --scheme {scheme} --key-type {key_type} --suri '{secret_phrase}'"
	);
	let _ = context.run_command(&cmd)?;
	let store_path = format!("{}/{}{public_key}", keystore_path, key_def.key_type_hex(),);
	context.eprint(&format!("💾 {name} key stored at {store_path}",));
	Ok(())
}

fn generate_or_load_key<C: IOContext>(
	config: &GenerateKeysConfig,
	context: &C,
	chain_spec_path: &str,
	key_def: &KeyDefinition,
) -> anyhow::Result<ByteString> {
	let keystore_path = config.keystore_path();
	let existing_keys = context.list_directory(&keystore_path)?.unwrap_or_default();

	let key: anyhow::Result<String> = if let Some(key) = find_existing_key(&existing_keys, key_def)
	{
		if context.prompt_yes_no(
			&format!("A {} key already exists in store: {key} - overwrite it?", key_def.name),
			false,
		) {
			let new_key = generate_keys(context, key_def)?;
			store_keys(context, config, key_def, &new_key, chain_spec_path)?;

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
		store_keys(context, config, key_def, &new_key, chain_spec_path)?;

		Ok(new_key.public_key)
	};
	ByteString::decode_hex(&key?).map_err(|e| anyhow!(e))
}
