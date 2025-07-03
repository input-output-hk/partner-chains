use crate::CmdRun;
use crate::config::KEYS_FILE_PATH;
use crate::io::IOContext;
use anyhow::{Context, Result, anyhow};
use clap::Parser;
use jsonrpsee::{core::client::ClientT, rpc_params, ws_client::WsClientBuilder};
use parity_scale_codec::Decode;
use serde_json::json;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

// Runtime API constants
const SESSION_KEYS_DECODE_API: &str = "SessionKeys_decode_session_keys";

#[derive(Clone, Debug, Parser)]
pub struct AutomaticGenerateKeysCmd {
	/// WebSocket endpoint URL for the substrate node
	#[arg(long, default_value = "ws://localhost:9933")]
	pub ws_endpoint: String,
}

#[derive(Debug)]
pub struct AutomaticGenerateKeysConfig {
	pub ws_endpoint: String,
}

impl From<AutomaticGenerateKeysCmd> for AutomaticGenerateKeysConfig {
	fn from(cmd: AutomaticGenerateKeysCmd) -> Self {
		Self { ws_endpoint: cmd.ws_endpoint }
	}
}

impl CmdRun for AutomaticGenerateKeysCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.eprint(
			"This 🧙 wizard will automatically generate session keys using the substrate node:",
		);
		context.eprint("→  Connect to the substrate node via WebSocket");
		context.eprint("→  Call author_rotateKeys to generate new session keys");
		context
			.eprint("→  Decode the session keys to extract individual key types and public keys");
		context.eprint("→  Store the keys in a JSON file for future use");
		context.enewline();

		let config = AutomaticGenerateKeysConfig::from(self.clone());

		// Use tokio runtime to handle async operations
		let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

		rt.block_on(async { generate_and_store_keys(&config, context).await })?;

		context.enewline();
		context.eprint("🚀 All done!");

		Ok(())
	}
}

async fn generate_and_store_keys<C: IOContext>(
	config: &AutomaticGenerateKeysConfig,
	context: &C,
) -> Result<()> {
	context.eprint(&format!("🔗 Connecting to substrate node at {}", config.ws_endpoint));

	// Connect to a Substrate node (following the user's example pattern)
	let client = WsClientBuilder::default()
		.build(&config.ws_endpoint)
		.await
		.context("Failed to connect to substrate node")?;

	context.eprint("✅ Connected to substrate node");

	// Step 1: Make the author_rotateKeys RPC call (following the user's example pattern)
	context.eprint("🔄 Rotating session keys...");
	let session_keys_hex: String = client
		.request("author_rotateKeys", rpc_params![])
		.await
		.context("Failed to rotate keys")?;

	// Convert hex string to bytes (following the user's example pattern)
	let session_keys = hex::decode(session_keys_hex.trim_start_matches("0x"))
		.context("Failed to decode session keys hex")?;

	context.eprint(&format!("🔑 Raw session keys generated: 0x{}", hex::encode(&session_keys)));

	// Step 2: Call SessionKeys_decode_session_keys runtime API (following the user's example pattern)
	context.eprint("🔍 Decoding session keys...");

	// Encode the session keys for the runtime API call
	let encoded_keys = hex::encode(&session_keys);

	// Call the runtime API to decode session keys
	let decoded_result_hex: String = client
		.request("state_call", rpc_params![SESSION_KEYS_DECODE_API, format!("0x{}", encoded_keys)])
		.await
		.context("Failed to call decode_session_keys runtime API")?;

	// Convert hex result to bytes and decode (following the user's example pattern)
	let decoded_result_bytes = hex::decode(decoded_result_hex.trim_start_matches("0x"))
		.context("Failed to decode result hex")?;

	// Decode the result as Vec<(Vec<u8>, Vec<u8>)> (following the user's example pattern)
	let decoded_keys: Vec<(Vec<u8>, Vec<u8>)> =
		Vec::<(Vec<u8>, Vec<u8>)>::decode(&mut decoded_result_bytes.as_slice())
			.context("Failed to decode session keys result")?;

	context.eprint(&format!("📋 Decoded {} key pairs", decoded_keys.len()));

	// Step 3: Convert decoded keys into a dictionary
	let mut key_map: HashMap<String, String> = HashMap::new();
	for (key_type_bytes, public_key_bytes) in decoded_keys {
		// Convert 4-byte key_type to string (e.g., b"aura" -> "aura")
		let key_type_str = String::from_utf8(key_type_bytes.clone())
			.map_err(|e| anyhow!("Invalid key type encoding: {}", e))?;

		// Convert public key to hex string
		let public_key_hex = format!("0x{}", hex::encode(&public_key_bytes));

		context.eprint(&format!("🔐 Found key: {} -> {}", key_type_str, public_key_hex));
		key_map.insert(key_type_str, public_key_hex);
	}

	// Step 4: Store the keys as JSON in a file
	let keys_json = json!(key_map);
	let pretty_json =
		serde_json::to_string_pretty(&keys_json).context("Failed to serialize keys to JSON")?;

	context.write_file(KEYS_FILE_PATH, &pretty_json);

	context.eprint(&format!("💾 Session keys saved to {} file:", KEYS_FILE_PATH));
	context.print(&pretty_json);
	context.eprint("These keys are now ready for use with your partner chain node.");

	Ok(())
}
