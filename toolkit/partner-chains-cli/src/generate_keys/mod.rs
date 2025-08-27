use self::config::KEYS_FILE_PATH;
use crate::io::IOContext;
use crate::keystore::*;
use crate::permissioned_candidates::PermissionedCandidateKeys;
use crate::{config::config_fields, *};
use anyhow::{Context, anyhow};
use parity_scale_codec::{Decode, Encode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
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
	/// URL of the Substrate node RPC endpoint for automatic key generation (e.g., http://localhost:9933).
	/// If not provided, keys will be generated locally using the traditional method.
	#[arg(long = "url")]
	node_url: Option<String>,
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
		match &self.node_url {
			Some(url) => {
				// Automatic key generation via RPC
				context.eprint("This 🧙 wizard will generate session keys by calling author_rotateKeys on the node, decode them, and save them to the keystore and partner-chains-public-keys.json file:");
				context.enewline();

				let config = GenerateKeysConfig::load(context);
				let keystore_path = keystore_path(&config.substrate_node_base_path);
				context.eprint(&format!("🔑 Keystore path: {}", keystore_path));
				context.enewline();

				let rt = tokio::runtime::Runtime::new()?;
				rt.block_on(async {
					let client = Client::new();

					// Step 1: Call author_rotateKeys RPC to get session keys
					let session_keys_hex =
						call_author_rotate_keys(&client, url, context).await?;

					// Step 2: Decode session keys using runtime API
					let decoded_keys =
						decode_session_keys(&client, url, &session_keys_hex, context).await?;

					// Step 3: Save keys to keystore and JSON file
					save_keys_to_storage(&decoded_keys, &session_keys_hex, &keystore_path, context).await?;

					context.print("🚀 All done!");
					Ok(())
				})
			},
			None => {
				// Traditional key generation
				context.eprint(
					"This 🧙 wizard will generate the following keys and save them to your node's keystore:",
				);
				context.eprint(&format!("→ {} {} key", CROSS_CHAIN.scheme, CROSS_CHAIN.name));
				for key_def in T::key_definitions() {
					context.eprint(&format!("→ {} {} key", key_def.scheme, key_def.name));
				}
				context.eprint("It will also generate a network key for your node if needed.");
				context.enewline();

				// Create a proper temporary chain spec as it was in master
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
	}
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

// JSON-RPC structures for automatic key generation
#[derive(Serialize)]
struct JsonRpcRequest {
	jsonrpc: String,
	method: String,
	params: serde_json::Value,
	id: u64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct JsonRpcResponse<T> {
	jsonrpc: String,
	result: Option<T>,
	error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
	code: i32,
	message: String,
}

/// Call author_rotateKeys RPC method to generate new session keys
async fn call_author_rotate_keys<C: IOContext>(
	client: &Client,
	node_url: &str,
	context: &C,
) -> anyhow::Result<String> {
	let session_keys_hex: String =
		send_rpc_request(client, node_url, "author_rotateKeys", serde_json::json!([]))
			.await
			.map_err(|e| anyhow!("Failed to call author_rotateKeys: {}", e))?;

	context.print(&format!("Raw session keys (hex): {}", session_keys_hex));
	Ok(session_keys_hex)
}

/// Decode session keys using the runtime API
async fn decode_session_keys<C: IOContext>(
	client: &Client,
	node_url: &str,
	session_keys_hex: &str,
	_context: &C,
) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
	// Decode hex string to bytes (remove "0x" prefix)
	let session_keys = hex::decode(&session_keys_hex[2..])
		.map_err(|e| anyhow!("Failed to decode session keys: {}", e))?;

	// Get finalized block hash
	let block_hash: String =
		send_rpc_request(client, node_url, "chain_getFinalizedHead", serde_json::json!([]))
			.await
			.map_err(|e| anyhow!("Failed to get finalized block hash: {}", e))?;

	// Use SCALE-encoded parameter for modern Polkadot SDK method
	let session_keys_param = format!("0x{}", hex::encode(session_keys.encode()));
	let params =
		serde_json::json!(["SessionKeys_decode_session_keys", session_keys_param, block_hash]);

	let decoded_keys: Vec<(Vec<u8>, Vec<u8>)> =
		match send_rpc_request::<String>(client, node_url, "state_call", params).await {
			Ok(decoded_hex) => {
				let bytes = hex::decode(&decoded_hex[2..])
					.map_err(|e| anyhow!("Failed to decode runtime API response: {}", e))?;

				parse_decoded_keys_response(&bytes)?
			},
			Err(e) => {
				return Err(anyhow!(
					"Failed to call SessionKeys_decode_session_keys: {}",
					e
				));
			},
		};

	Ok(decoded_keys)
}

/// Parse the SCALE-encoded response from the runtime API
fn parse_decoded_keys_response(bytes: &[u8]) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
	// Try decoding as Option<Vec<(Vec<u8>, u32)>> (newer Polkadot SDK)
	let mut cursor = bytes;
	match <Option<Vec<(Vec<u8>, u32)>>>::decode(&mut cursor) {
		Ok(Some(vec)) if cursor.is_empty() => {
			return Ok(vec
				.into_iter()
				.map(|(pubkey, key_type)| (key_type.to_le_bytes().to_vec(), pubkey))
				.collect());
		},
		Ok(None) if cursor.is_empty() => {
			// Successfully decoded as None (empty result)
			return Ok(Vec::new());
		},
		_ => {
			// Try Vec<(Vec<u8>, Vec<u8>)> (legacy format)
			let mut cursor_alt = bytes;
			match <Vec<(Vec<u8>, Vec<u8>)>>::decode(&mut cursor_alt) {
				Ok(vec) if cursor_alt.is_empty() => return Ok(vec),
				_ => {
					// Try Option<Vec<(Vec<u8>, Vec<u8>)>> (alternative legacy)
					let mut cursor_opt = bytes;
					match <Option<Vec<(Vec<u8>, Vec<u8>)>>>::decode(&mut cursor_opt) {
						Ok(Some(vec)) if cursor_opt.is_empty() => Ok(vec),
						Ok(None) if cursor_opt.is_empty() => Ok(Vec::new()),
						_ => {
							return Err(anyhow!("Failed to SCALE decode keys"));
						},
					}
				},
			}
		},
	}
}

/// Save keys to keystore and JSON file
async fn save_keys_to_storage<C: IOContext>(
	decoded_keys: &[(Vec<u8>, Vec<u8>)],
	session_keys_hex: &str,
	keystore_path: &str,
	context: &C,
) -> anyhow::Result<()> {
	// Create keystore directory - in tests this is mocked, in real usage it creates the directory
	let _ = context.run_command(&format!("mkdir -p {}", keystore_path));

	let mut keys: BTreeMap<String, ByteString> = BTreeMap::new();

	if !decoded_keys.is_empty() {
		save_decoded_keys(decoded_keys, keystore_path, &mut keys, context)?;
	} else {
		save_raw_keys_as_fallback(session_keys_hex, keystore_path, &mut keys, context)?;
	}

	// For automatic key generation, generate the cross-chain key without requiring a chain spec
	let partner_chains_key = generate_cross_chain_key_for_automatic_flow(context, keystore_path)?;

	// Create PermissionedCandidateKeys struct to match traditional generate-keys format
	let permissioned_keys = PermissionedCandidateKeys {
		partner_chains_key,
		keys: keys.clone(),
	};

	save_permissioned_keys_to_json_file(&permissioned_keys, context)?;

	// Print decoded keys for reference
	context.print(&format!("Decoded session keys: {:?}", keys));
	context.eprint("Note: Cross-chain key has been generated and included in the JSON file.");

	Ok(())
}

/// Generate cross-chain key for automatic flow without requiring a chain spec file
fn generate_cross_chain_key_for_automatic_flow<C: IOContext>(
	context: &C,
	keystore_path: &str,
) -> anyhow::Result<ByteString> {
	// Check if cross-chain key already exists
	let existing_keys = context.list_directory(keystore_path)?.unwrap_or_default();
	
	if let Some(existing_key) = find_existing_key(&existing_keys, &CROSS_CHAIN) {
		if context.prompt_yes_no(
			&format!("A {} key already exists in store: {} - overwrite it?", CROSS_CHAIN.name, existing_key),
			false,
		) {
			// Generate new key
			let new_key = generate_keys(context, &CROSS_CHAIN)?;
			
			// Save to keystore directly (without chain spec)
			let store_path = format!("{}/{}{}", keystore_path, CROSS_CHAIN.key_type_hex(), new_key.public_key);
			context.write_file(&store_path, &new_key.public_key);
			context.eprint(&format!("💾 {} key stored at {}", CROSS_CHAIN.name, store_path));
			
			// Remove old key if it exists
			let old_key_path = format!("{}/{}{}", keystore_path, CROSS_CHAIN.key_type_hex(), existing_key);
			if context.file_exists(&old_key_path) {
				context.delete_file(&old_key_path)?;
			}
			
			ByteString::decode_hex(&new_key.public_key)
				.map_err(|e| anyhow!("Failed to decode hex: {}", e))
		} else {
			// Use existing key
			ByteString::decode_hex(&format!("0x{}", existing_key))
				.map_err(|e| anyhow!("Failed to decode hex: {}", e))
		}
	} else {
		// Generate new key
		let new_key = generate_keys(context, &CROSS_CHAIN)?;
		
		// Save to keystore directly (without chain spec)
		let store_path = format!("{}/{}{}", keystore_path, CROSS_CHAIN.key_type_hex(), new_key.public_key);
		context.write_file(&store_path, &new_key.public_key);
		context.eprint(&format!("💾 {} key stored at {}", CROSS_CHAIN.name, store_path));
		
		ByteString::decode_hex(&new_key.public_key)
			.map_err(|e| anyhow!("Failed to decode hex: {}", e))
	}
}

/// Save successfully decoded keys to keystore
fn save_decoded_keys<C: IOContext>(
	decoded_keys: &[(Vec<u8>, Vec<u8>)],
	keystore_path: &str,
	keys: &mut BTreeMap<String, ByteString>,
	context: &C,
) -> anyhow::Result<()> {
	for (key_type, public_key) in decoded_keys {
		// Convert key type to string for JSON and display
		let key_type_str = String::from_utf8(key_type.clone())
			.map_err(|e| anyhow!("Invalid key type encoding: {}", e))?;

		// Save to keystore with key_type_hex + public_key format
		let key_type_hex = hex::encode(key_type);
		let store_path = format!("{}/{}{}", keystore_path, key_type_hex, hex::encode(public_key));
		context.write_file(&store_path, &hex::encode(public_key));
		context.print(&format!("Saved {} key to {}", key_type_str, store_path));

		// Store in keys map for JSON output as ByteString
		keys.insert(key_type_str, ByteString::from(public_key.clone()));
	}
	Ok(())
}

/// Save raw session keys as fallback when decoding fails
fn save_raw_keys_as_fallback<C: IOContext>(
	session_keys_hex: &str,
	keystore_path: &str,
	keys: &mut BTreeMap<String, ByteString>,
	context: &C,
) -> anyhow::Result<()> {
	context.eprint("⚠️ No session keys decoded. Saving raw keys as fallback.");
	context.eprint("Please verify the node's runtime configuration by fetching metadata:");
	context.eprint("curl -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"state_getMetadata\",\"id\":1}' http://localhost:9933 > metadata.json");
	context.eprint("Look for the Session pallet and SessionKeys type to determine key order (e.g., aura, gran, imon).");

	let session_keys = hex::decode(&session_keys_hex[2..])
		.map_err(|e| anyhow!("Failed to decode session keys: {}", e))?;

	let store_path = format!("{}/raw{}", keystore_path, hex::encode(&session_keys));
	context.write_file(&store_path, &hex::encode(&session_keys));
	context.print(&format!("Saved raw session keys to {}", store_path));
	keys.insert("raw".to_string(), ByteString::from(session_keys));

	Ok(())
}

/// Save keys to JSON file
fn save_permissioned_keys_to_json_file<C: IOContext>(
	permissioned_keys: &PermissionedCandidateKeys,
	context: &C,
) -> anyhow::Result<()> {
	if !permissioned_keys.keys.is_empty() || !permissioned_keys.partner_chains_key.0.is_empty() {
		if prompt_can_write("keys file", KEYS_FILE_PATH, context) {
			let public_keys_json = serde_json::to_string_pretty(permissioned_keys)
				.map_err(|e| anyhow!("Failed to serialize public keys: {}", e))?;
			context.write_file(KEYS_FILE_PATH, &public_keys_json);
			
			context.eprint(&format!(
				"🔑 The following public keys were generated and saved to the {} file:",
				KEYS_FILE_PATH,
			));
			context.print(&public_keys_json);
			context.eprint("You may share them with your chain governance authority");
			context.eprint("if you wish to be included as a permissioned candidate.");
		} else {
			context.eprint("Refusing to overwrite keys file - skipping");
		}
	} else {
		context.eprint("Warning: No keys to save, skipping JSON file creation");
	}
	Ok(())
}

/// Helper to send a JSON-RPC request
async fn send_rpc_request<T: for<'de> Deserialize<'de>>(
	client: &Client,
	url: &str,
	method: &str,
	params: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error>> {
	let request =
		JsonRpcRequest { jsonrpc: "2.0".to_string(), method: method.to_string(), params, id: 1 };

	let response = client
		.post(url)
		.json(&request)
		.send()
		.await?
		.json::<JsonRpcResponse<T>>()
		.await?;

	if let Some(error) = response.error {
		return Err(format!("RPC error: {} (code: {})", error.message, error.code).into());
	}

	response.result.ok_or_else(|| "No result in response".into())
}

fn write_temp_chain_spec<C: IOContext>(context: &C, chain_spec: serde_json::Value) -> String {
	let dir_path = context.new_tmp_dir();
	let dir_path = dir_path.to_str().expect("temp dir path is correct utf-8");
	let path = format!("{dir_path}/chain-spec.json");
	let content = format!("{chain_spec}");
	context.write_file(&path, &content);
	path
}
