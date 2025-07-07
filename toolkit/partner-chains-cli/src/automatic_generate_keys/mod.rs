use crate::generate_keys::GenerateKeysConfig;
use crate::keystore::keystore_path;
use crate::{CmdRun, IOContext};
use clap::Parser;
use parity_scale_codec::{Decode, Encode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

#[cfg(test)]
mod tests;

const KEYS_FILE_PATH: &str = "./keys.json";

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
	#[arg(long, default_value = "http://localhost:9933")]
	node_url: String,
}

impl CmdRun for AutomaticGenerateKeysCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.eprint("This 🧙 wizard will generate session keys by calling author_rotateKeys on the node, decode them, and save them to the keystore and keys.json file:");
		context.enewline();

		let config = GenerateKeysConfig::load(context);
		let keystore_path = keystore_path(&config.substrate_node_base_path);
		context.eprint(&format!("🔑 Keystore path: {}", keystore_path));
		context.enewline();

		let rt = tokio::runtime::Runtime::new()?;
		rt.block_on(async {
			let client = Client::new();

			// Step 1: Call author_rotateKeys RPC.
			let session_keys_hex: String = send_rpc_request(
				&client,
				&self.node_url,
				"author_rotateKeys",
				serde_json::json!([]),
			)
			.await
			.map_err(|e| anyhow::anyhow!("Failed to call author_rotateKeys: {}", e))?;
			context.print(&format!("Raw session keys (hex): {}", session_keys_hex));

			// Decode hex string to bytes (remove "0x" prefix).
			let session_keys = hex::decode(&session_keys_hex[2..])
				.map_err(|e| anyhow::anyhow!("Failed to decode session keys: {}", e))?;

			// Step 2: Decode the session keys using the runtime API.
			let block_hash: String = send_rpc_request(
				&client,
				&self.node_url,
				"chain_getFinalizedHead",
				serde_json::json!([]),
			)
			.await
			.map_err(|e| anyhow::anyhow!("Failed to get finalized block hash: {}", e))?;

			let session_keys_param_raw = format!("0x{}", hex::encode(&session_keys));
			// For the new API we need SCALE-encode the Vec<u8> first (compact length prefix).
			let session_keys_param_scale = format!("0x{}", hex::encode(session_keys.encode()));

			// Newer Substrate versions expose the method under `SessionKeys_decode_session_keys`.
			// Some older versions still use `Session_decodeSessionKeys`. Try both. We call the
			// legacy one first to remain compatible with existing tests and most nodes.
			let decode_methods = [
				"Session_decodeSessionKeys",       // legacy (Polkadot/Substrate <= v0.9)
				"SessionKeys_decode_session_keys", // newer API (after FRAME v1.0)
			];

			let mut decoded_keys: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
			let mut last_err: Option<anyhow::Error> = None;

			for decode_method in &decode_methods {
				let param = if *decode_method == "Session_decodeSessionKeys" {
					&session_keys_param_raw
				} else {
					&session_keys_param_scale
				};

				let params = serde_json::json!([decode_method, param, block_hash]);
				match send_rpc_request::<String>(&client, &self.node_url, "state_call", params)
					.await
				{
					Ok(decoded_hex) => {
						let bytes = hex::decode(&decoded_hex[2..]).map_err(|e| {
							anyhow::anyhow!("Failed to decode runtime API response: {}", e)
						})?;
						// The runtime API may return:
						// 1. Vec<(Vec<u8>, Vec<u8>)>  – legacy order (key_type bytes, pubkey)
						// 2. Vec<(Vec<u8>, KeyTypeId)> – current sp_session order (pubkey, KeyTypeId)
						// 3. Option<Vec<..>> wrapper  – newer FRAME versions add Option
						// Try each pattern progressively.
						decoded_keys = {
							// Attempt direct Vec decode first.
							let mut cursor = &bytes[..];
							match <Vec<(Vec<u8>, Vec<u8>)>>::decode(&mut cursor) {
								Ok(vec) if cursor.is_empty() => vec,
								_ => {
									// Try Vec<(Vec<u8>, u32)> where u32 is KeyTypeId
									let mut cursor_alt = &bytes[..];
									if let Ok(vec_u32) =
										<Vec<(Vec<u8>, u32)>>::decode(&mut cursor_alt)
									{
										vec_u32
											.into_iter()
											.map(|(pubkey, key_type)| {
												(key_type.to_le_bytes().to_vec(), pubkey)
											})
											.collect::<Vec<_>>()
									} else {
										// Fallback: try Option<Vec<(Vec<u8>, Vec<u8>)>>
										let mut cursor_opt = &bytes[..];
										<Option<Vec<(Vec<u8>, Vec<u8>)>>>::decode(&mut cursor_opt)
											.map_err(|e| {
												anyhow::anyhow!(
													"Failed to SCALE decode Option<Vec> keys: {}",
													e
												)
											})?
											.unwrap_or_default()
									}
								},
							}
						};
						// Successfully decoded.
						last_err = None;
						break;
					},
					Err(e) => {
						// Capture error and try the next variant.
						last_err = Some(anyhow::anyhow!("{}", e));
					},
				}
			}

			// If no keys were decoded AND all RPC attempts errored, propagate the last error.
			if decoded_keys.is_empty() {
				if let Some(err) = last_err {
					return Err(err);
				}
				// Otherwise (successful call but empty result) continue; downstream logic will
				// handle the empty key map and emit a warning instead.
			}

			// Step 3: Process and save keys.
			fs::create_dir_all(&keystore_path)
				.map_err(|e| anyhow::anyhow!("Failed to create keystore directory: {}", e))?;

			let mut key_map: BTreeMap<String, String> = BTreeMap::new();

			for (first, second) in decoded_keys {
				// The runtime can return (public_key, key_type) or (key_type, public_key)
				// depending on its version. Determine which is which by length (key_type is 4 bytes).
				let (key_type_bytes, public_key) = if first.len() == 4 {
					(first.clone(), second.clone())
				} else if second.len() == 4 {
					(second.clone(), first.clone())
				} else {
					// Unknown tuple layout; skip.
					continue;
				};

				let key_type_str = String::from_utf8(key_type_bytes.clone())
					.map_err(|e| anyhow::anyhow!("Invalid key type encoding: {}", e))?;
				let public_key_hex = format!("0x{}", hex::encode(&public_key));

				// Save to keystore with key_type_hex + public_key format.
				let key_type_hex = hex::encode(&key_type_bytes);
				let store_path =
					format!("{}/{}{}", keystore_path, key_type_hex, hex::encode(&public_key));
				fs::write(&store_path, &public_key)
					.map_err(|e| anyhow::anyhow!("Failed to write key to {}: {}", store_path, e))?;
				context.print(&format!("Saved {} key to {}", key_type_str, store_path));

				key_map.insert(key_type_str, public_key_hex);
			}

			// Step 4: Save all keys to keys.json.
			if !key_map.is_empty() {
				if prompt_can_write("keys file", KEYS_FILE_PATH, context) {
					let public_keys_json = serde_json::to_string_pretty(&key_map)
						.map_err(|e| anyhow::anyhow!("Failed to serialize public keys: {}", e))?;
					context.write_file(KEYS_FILE_PATH, &public_keys_json);
					context.print(&format!(
						"🔑 Public keys saved to {}:\n{}",
						KEYS_FILE_PATH, public_keys_json
					));
					context.print(
						"You may share these public keys with your chain governance authority.",
					);
				} else {
					context.print("Refusing to overwrite keys file - skipping JSON save");
				}
			} else {
				context.print("Warning: No keys decoded, skipping JSON save");
			}

			// Print decoded keys for reference.
			context.print(&format!("Decoded session keys: {:?}", key_map));

			context.print("🚀 All done!");
			Ok(())
		})
	}
}

// Helper to prompt if the keys file can be written.
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

// Helper to send a JSON-RPC request.
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
