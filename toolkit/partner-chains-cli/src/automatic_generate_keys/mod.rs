use crate::io::IOContext;
use crate::*;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use reqwest;

#[cfg(test)]
mod tests;

// We'll use dynamic metadata handling instead of static code generation
// This allows us to work with any Partner Chain runtime without pre-generated metadata

#[derive(Clone, Debug, clap::Parser)]
pub struct AutomaticGenerateKeysCmd {
	/// Substrate node RPC URL
	#[arg(long, default_value = "http://localhost:9944")]
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

	let json_response: JsonRpcResponse = response
		.json()
		.context("Failed to parse RPC response")?;

	let keys_hex = json_response.result
		.ok_or_else(|| anyhow!("No result in RPC response: {:?}", json_response.error))?;

	context.eprint(&format!("✅ Generated session keys: {}", keys_hex));

	// Step 2: Decode session keys
	context.eprint("🔍 Decoding session keys to get key types...");

	let decode_request = JsonRpcRequest {
		jsonrpc: "2.0".to_string(),
		method: "sessionKeys_decodeSessionKeys".to_string(),
		params: vec![keys_hex.clone()],
		id: 2,
	};

	let decode_response = client
		.post(&config.node_url)
		.header("Content-Type", "application/json")
		.json(&decode_request)
		.send()
		.context("Failed to send decode RPC request")?;

	let decode_json: serde_json::Value = decode_response
		.json()
		.context("Failed to parse decode response")?;

	context.eprint(&format!("✅ Decode response: {}", decode_json.to_string()));

	// Extract the array from the response
	let key_array = decode_json
		.get("result")
		.and_then(|v| v.as_array())
		.ok_or_else(|| anyhow!("Expected array in decode response"))?;

	let mut session_keys = Vec::new();

	for (index, key_entry) in key_array.iter().enumerate() {
		let key_pair = key_entry
			.as_array()
			.ok_or_else(|| anyhow!("Expected array for key entry {}", index))?;

		if key_pair.len() != 2 {
			return Err(anyhow!("Expected key entry {} to have 2 elements", index));
		}

		let public_key_hex = key_pair[0]
			.as_str()
			.ok_or_else(|| anyhow!("Expected string for public key in entry {}", index))?;

		let key_type = key_pair[1]
			.as_str()
			.ok_or_else(|| anyhow!("Expected string for key type in entry {}", index))?;

		session_keys.push(SessionKeyInfo {
			key_type: key_type.to_string(),
			public_key: public_key_hex.to_string(),
		});
	}

	context.eprint(&format!("✅ Successfully decoded {} session keys", session_keys.len()));

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
