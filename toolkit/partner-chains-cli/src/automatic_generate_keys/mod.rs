use crate::io::IOContext;
use crate::*;
use anyhow::{Context, Result, anyhow};
use reqwest;
use serde::{Deserialize, Serialize};
use subxt::{OnlineClient, SubstrateConfig, metadata::DecodeWithMetadata};
use tokio;

#[cfg(test)]
mod tests;

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
		let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
		rt.block_on(generate_keys_via_rpc(&config, context))
	}
}

async fn generate_keys_via_rpc<C: IOContext>(
	config: &AutomaticGenerateKeysConfig,
	context: &C,
) -> Result<()> {
	context.eprint("🔑 Generating session keys via RPC...");

	// Step 1: Generate session keys using JSON-RPC
	let client = reqwest::Client::new();
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
		.await
		.context("Failed to send RPC request")?;

	let json_response: JsonRpcResponse =
		response.json().await.context("Failed to parse RPC response")?;

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

	// Step 2: Get session key types from metadata
	context.eprint("🔍 Fetching session key types from metadata...");
	let key_types = get_session_key_types(&config.node_url).await.unwrap_or_else(|_| {
		context.eprint("  ⚠️  Failed to fetch metadata, using default key types");
		vec!["aura".to_string(), "gran".to_string()]
	});

	context.eprint(&format!("📝 Found key types: {:?}", key_types));

	// Step 3: Parse session keys
	context.eprint("🔍 Parsing session keys...");
	let session_keys = parse_session_keys_hex(&keys_hex, &key_types, context)?;

	context.eprint(&format!("✅ Successfully parsed {} session keys", session_keys.len()));

	// Step 4: Save to JSON file
	let output_path = "session_keys.json";
	let json_output = serde_json::to_string_pretty(&session_keys)
		.context("Failed to serialize session keys to JSON")?;

	if prompt_can_write("session_keys file", output_path, context) {
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

/// Fetch session key types from node metadata using subxt
async fn get_session_key_types(url: &str) -> Result<Vec<String>> {
	let client = OnlineClient::<SubstrateConfig>::from_url(url)
		.await
		.context("Failed to create subxt client")?;
	let metadata = client.metadata();

	// Look up the SessionKeys type in the metadata
	let type_id = metadata
		.types()
		.types()
		.iter()
		.find(|ty| ty.ty().path().segments().last() == Some("SessionKeys"))
		.map(|ty| ty.id())
		.ok_or_else(|| anyhow!("SessionKeys type not found in metadata"))?;

	// Decode the type to get the tuple of key types
	let ty = metadata
		.types()
		.resolve(type_id)
		.context("Failed to resolve SessionKeys type")?;
	let key_types = match ty.type_def() {
		scale_type_resolver::TypeDef::Tuple(tuple) => tuple
			.fields()
			.iter()
			.filter_map(|field| {
				let ty = metadata.types().resolve(*field).ok()?;
				let path = ty.path();
				path.segments().last().map(|s| s.to_string())
			})
			.collect::<Vec<String>>(),
		_ => return Err(anyhow!("SessionKeys type is not a tuple")),
	};

	Ok(key_types)
}

/// Parse session keys from hex string using dynamic key types
fn parse_session_keys_hex<C: IOContext>(
	keys_hex: &str,
	key_types: &[String],
	context: &C,
) -> Result<Vec<SessionKeyInfo>> {
	// Validate hex string
	let hex_data = keys_hex.strip_prefix("0x").unwrap_or(keys_hex);
	if hex_data.len() % 2 != 0 || !hex_data.chars().all(|c| c.is_ascii_hexdigit()) {
		return Err(anyhow!("Invalid hex string: {}", keys_hex));
	}

	// Assume 32-byte keys (64 hex chars) for now; could be made dynamic via metadata
	let key_length = 64;
	let mut session_keys = Vec::new();
	let mut offset = 0;

	for key_type in key_types {
		if offset + key_length <= hex_data.len() {
			let key_hex = &hex_data[offset..offset + key_length];
			session_keys.push(SessionKeyInfo {
				key_type: key_type.to_string(),
				public_key: format!("0x{}", key_hex),
			});
			context.eprint(&format!("  📝 Parsed {} key: 0x{}", key_type, key_hex));
			offset += key_length;
		} else {
			context.eprint(&format!("  ⚠️  Not enough data for key type: {}", key_type));
			break;
		}
	}

	// Handle remaining data
	if offset < hex_data.len() {
		let remaining_hex = &hex_data[offset..];
		session_keys.push(SessionKeyInfo {
			key_type: "remaining".to_string(),
			public_key: format!("0x{}", remaining_hex),
		});
		context.eprint(&format!("  📝 Remaining data: 0x{}", remaining_hex));
	}

	// Fallback if no keys were parsed
	if session_keys.is_empty() {
		context.eprint("  ⚠️  Could not parse individual keys - providing full hex as raw");
		session_keys
			.push(SessionKeyInfo { key_type: "raw".to_string(), public_key: keys_hex.to_string() });
	}

	Ok(session_keys)
}
