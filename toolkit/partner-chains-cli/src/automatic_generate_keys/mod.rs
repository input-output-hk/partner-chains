use crate::config::KEYS_FILE_PATH;
use crate::generate_keys::GenerateKeysConfig;
use crate::keystore::keystore_path;
use crate::{CmdRun, IOContext};
use clap::Parser;
use indexmap::IndexMap;
use parity_scale_codec::{Decode, Encode};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

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

/// Command to automatically generate and save session keys by connecting to a node.
#[derive(Clone, Debug, Parser)]
pub struct AutomaticGenerateKeysCmd {
	/// URL of the Substrate node RPC endpoint (e.g., http://localhost:9933).
	#[arg(long = "url", default_value = "http://localhost:9933")]
	node_url: String,
}

impl CmdRun for AutomaticGenerateKeysCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
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
				call_author_rotate_keys(&client, &self.node_url, context).await?;

			// Step 2: Decode session keys using runtime API
			let decoded_keys =
				decode_session_keys(&client, &self.node_url, &session_keys_hex, context).await?;

			// Step 3: Save keys to keystore and JSON file
			save_keys_to_storage(&decoded_keys, &session_keys_hex, &keystore_path, context).await?;

			context.print("🚀 All done!");
			Ok(())
		})
	}
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
			.map_err(|e| anyhow::anyhow!("Failed to call author_rotateKeys: {}", e))?;

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
		.map_err(|e| anyhow::anyhow!("Failed to decode session keys: {}", e))?;

	// Get finalized block hash
	let block_hash: String =
		send_rpc_request(client, node_url, "chain_getFinalizedHead", serde_json::json!([]))
			.await
			.map_err(|e| anyhow::anyhow!("Failed to get finalized block hash: {}", e))?;

	// Use SCALE-encoded parameter for modern Polkadot SDK method
	let session_keys_param = format!("0x{}", hex::encode(session_keys.encode()));
	let params =
		serde_json::json!(["SessionKeys_decode_session_keys", session_keys_param, block_hash]);

	let decoded_keys: Vec<(Vec<u8>, Vec<u8>)> =
		match send_rpc_request::<String>(client, node_url, "state_call", params).await {
			Ok(decoded_hex) => {
				let bytes = hex::decode(&decoded_hex[2..])
					.map_err(|e| anyhow::anyhow!("Failed to decode runtime API response: {}", e))?;

				parse_decoded_keys_response(&bytes)?
			},
			Err(e) => {
				return Err(anyhow::anyhow!(
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
						Ok(Some(vec)) if cursor_opt.is_empty() => return Ok(vec),
						Ok(None) if cursor_opt.is_empty() => return Ok(Vec::new()),
						_ => {
							return Err(anyhow::anyhow!("Failed to SCALE decode keys"));
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

	let mut key_map: IndexMap<String, String> = IndexMap::new();

	if !decoded_keys.is_empty() {
		save_decoded_keys(decoded_keys, keystore_path, &mut key_map, context)?;
	} else {
		save_raw_keys_as_fallback(session_keys_hex, keystore_path, &mut key_map, context)?;
	}

	save_keys_to_json_file(&key_map, context)?;

	// Print decoded keys for reference
	context.print(&format!("Decoded session keys: {:?}", key_map));

	Ok(())
}

/// Save successfully decoded keys to keystore
fn save_decoded_keys<C: IOContext>(
	decoded_keys: &[(Vec<u8>, Vec<u8>)],
	keystore_path: &str,
	key_map: &mut IndexMap<String, String>,
	context: &C,
) -> anyhow::Result<()> {
	for (key_type, public_key) in decoded_keys {
		// Convert key type to string for JSON and display
		let key_type_str = String::from_utf8(key_type.clone())
			.map_err(|e| anyhow::anyhow!("Invalid key type encoding: {}", e))?;
		let public_key_hex = format!("0x{}", hex::encode(public_key));

		// Save to keystore with key_type_hex + public_key format
		let key_type_hex = hex::encode(key_type);
		let store_path = format!("{}/{}{}", keystore_path, key_type_hex, hex::encode(public_key));
		context.write_file(&store_path, &hex::encode(public_key));
		context.print(&format!("Saved {} key to {}", key_type_str, store_path));

		// Store in key map for JSON output
		key_map.insert(key_type_str, public_key_hex);
	}
	Ok(())
}

/// Save raw session keys as fallback when decoding fails
fn save_raw_keys_as_fallback<C: IOContext>(
	session_keys_hex: &str,
	keystore_path: &str,
	key_map: &mut IndexMap<String, String>,
	context: &C,
) -> anyhow::Result<()> {
	context.eprint("⚠️ No session keys decoded. Saving raw keys as fallback.");
	context.eprint("Please verify the node's runtime configuration by fetching metadata:");
	context.eprint("curl -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"state_getMetadata\",\"id\":1}' http://localhost:9933 > metadata.json");
	context.eprint("Look for the Session pallet and SessionKeys type to determine key order (e.g., aura, gran, imon).");

	let session_keys = hex::decode(&session_keys_hex[2..])
		.map_err(|e| anyhow::anyhow!("Failed to decode session keys: {}", e))?;

	let raw_key_hex = format!("0x{}", hex::encode(&session_keys));
	let store_path = format!("{}/raw{}", keystore_path, hex::encode(&session_keys));
	context.write_file(&store_path, &hex::encode(&session_keys));
	context.print(&format!("Saved raw session keys to {}", store_path));
	key_map.insert("raw".to_string(), raw_key_hex);

	Ok(())
}

/// Save keys to JSON file
fn save_keys_to_json_file<C: IOContext>(
	key_map: &IndexMap<String, String>,
	context: &C,
) -> anyhow::Result<()> {
	if !key_map.is_empty() {
		if prompt_can_write("keys file", KEYS_FILE_PATH, context) {
			let public_keys_json = serde_json::to_string_pretty(key_map)
				.map_err(|e| anyhow::anyhow!("Failed to serialize public keys: {}", e))?;
			context.write_file(KEYS_FILE_PATH, &public_keys_json);
			context.print(&format!(
				"🔑 Public keys saved to {}:\n{}",
				KEYS_FILE_PATH, public_keys_json
			));
			context.print("You may share these public keys with your chain governance authority.");
		} else {
			context.print("Refusing to overwrite keys file - skipping JSON save");
		}
	} else {
		context.print("Warning: No keys decoded, skipping JSON save");
	}
	Ok(())
}

/// Helper to prompt if the keys file can be written
fn prompt_can_write<C: IOContext>(file_desc: &str, file_path: &str, context: &C) -> bool {
	if context.file_exists(file_path) {
		context.prompt_yes_no(
			&format!("A {} already exists at {} - overwrite it?", file_desc, file_path),
			false,
		)
	} else {
		true
	}
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
