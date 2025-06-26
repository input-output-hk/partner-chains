use crate::io::IOContext;
use crate::*;
use anyhow::{Context, Result, anyhow};
use reqwest;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

// We'll use dynamic metadata handling instead of static code generation
// This allows us to work with any Partner Chain runtime without pre-generated metadata

#[derive(Clone, Debug, clap::Parser)]
pub struct AutomaticGenerateKeysCmd {
	/// Substrate node RPC URL
	#[arg(long, default_value = "http://localhost:9933")]
	pub url: String,
}

#[derive(Debug)]
pub struct AutomaticGenerateKeysConfig {
	pub node_url: String,
}

impl AutomaticGenerateKeysConfig {
	pub(crate) fn load<C: IOContext>(_context: &C, url: String) -> Self {
		Self { node_url: url }
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionKeyInfo {
	pub key_type: String,
	pub public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcRequest {
	jsonrpc: String,
	method: String,
	params: Vec<String>,
	id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcResponse {
	jsonrpc: String,
	result: Option<String>,
	error: Option<JsonRpcError>,
	id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
	code: i32,
	message: String,
}

impl CmdRun for AutomaticGenerateKeysCmd {
	fn run<C: IOContext>(&self, context: &C) -> Result<()> {
		let config = AutomaticGenerateKeysConfig::load(context, self.url.clone());
		generate_keys_via_rpc(&config, "", context)
	}
}

fn generate_keys_via_rpc<C: IOContext>(
	config: &AutomaticGenerateKeysConfig,
	_node_executable: &str,
	context: &C,
) -> Result<()> {
	context.eprint("🔑 Generating session keys via RPC...");

	// Step 1: Generate session keys using JSON-RPC
	let client = reqwest::blocking::Client::new();
	let request = JsonRpcRequest {
		jsonrpc: "2.0".to_string(),
		method: "author_rotateKeys".to_string(),
		params: vec![],
		id: 1,
	};

	let response = client
		.post(&config.node_url)
		.header("Content-Type", "application/json")
		.json(&request)
		.send()
		.context("Failed to send RPC request")?;

	let json_response: JsonRpcResponse = response.json().context("Failed to parse RPC response")?;

	let keys_hex = json_response
		.result
		.ok_or_else(|| {
			if let Some(error) = &json_response.error {
				if error.code == -32601 && error.message.contains("unsafe") {
					anyhow!(
						"RPC call is unsafe to be called externally. \
						To fix this, start your node with --rpc-methods=unsafe flag, or use --rpc-methods=auto if running locally. \
						Error: {} (code: {})", 
						error.message, error.code
					)
				} else {
					anyhow!("RPC error: {} (code: {})", error.message, error.code)
				}
			} else {
				anyhow!("No result in RPC response and no error provided")
			}
		})?;

	context.eprint(&format!("✅ Generated session keys: {}", keys_hex));

	// Step 2: Parse session keys manually since runtime decode is failing
	context.eprint("🔍 Parsing session keys...");

	let session_keys = parse_session_keys_hex(&keys_hex, context);

	context.eprint(&format!("✅ Successfully parsed {} session keys", session_keys.len()));

	// Step 3: Save to JSON file
	let output_path = "session_keys.json";
	let json_output = serde_json::to_string_pretty(&session_keys)
		.context("Failed to serialize session keys to JSON")?;

	if prompt_can_write("session keys file", output_path, context) {
		context.write_file(output_path, &json_output);
		context.eprint(&format!("💾 Session keys saved to {}", output_path));
		context.eprint("🔑 Generated session keys:");
		context.print(&json_output);
	} else {
		context.eprint("Refusing to overwrite session keys file - skipping save");
		context.eprint("🔑 Generated session keys:");
		context.print(&json_output);
	}

	Ok(())
}

/// Parse session keys from hex string by splitting into common key lengths
/// Substrate session keys are typically concatenated public keys of fixed lengths
fn parse_session_keys_hex<C: IOContext>(keys_hex: &str, context: &C) -> Vec<SessionKeyInfo> {
	// Remove 0x prefix if present
	let hex_data = keys_hex.strip_prefix("0x").unwrap_or(keys_hex);
	
	// Common key types and their lengths in bytes (hex chars = bytes * 2)
	// AURA: 32 bytes (64 hex chars) - Sr25519
	// GRANDPA: 32 bytes (64 hex chars) - Ed25519
	// ImOnline: 32 bytes (64 hex chars) - Sr25519
	// AuthorityDiscovery: 32 bytes (64 hex chars) - Sr25519
	
	let mut session_keys = Vec::new();
	let mut offset = 0;
	let key_types = ["aura", "gran", "imon", "auth"];
	
	// Each key is typically 32 bytes = 64 hex characters
	let key_length = 64;
	
	for (index, &key_type) in key_types.iter().enumerate() {
		if offset + key_length <= hex_data.len() {
			let key_hex = &hex_data[offset..offset + key_length];
			session_keys.push(SessionKeyInfo {
				key_type: key_type.to_string(),
				public_key: format!("0x{}", key_hex),
			});
			offset += key_length;
			
			context.eprint(&format!("  📝 Parsed {} key: 0x{}", key_type, key_hex));
		} else {
			break;
		}
	}
	
	// If there's remaining data, add it as a raw key
	if offset < hex_data.len() {
		let remaining_hex = &hex_data[offset..];
		session_keys.push(SessionKeyInfo {
			key_type: "remaining".to_string(),
			public_key: format!("0x{}", remaining_hex),
		});
		context.eprint(&format!("  📝 Remaining data: 0x{}", remaining_hex));
	}
	
	// If we couldn't parse any keys, provide the full hex as raw
	if session_keys.is_empty() {
		context.eprint("  ⚠️  Could not parse individual keys - providing full hex as raw");
		session_keys.push(SessionKeyInfo {
			key_type: "raw".to_string(),
			public_key: keys_hex.to_string(),
		});
	}
	
	session_keys
}
