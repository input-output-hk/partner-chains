use super::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use subxt::{OnlineClient, SubstrateConfig, dynamic::Value, backend::rpc::{RpcClient, RpcParams}};
use tokio;

#[derive(Clone, Debug, clap::Parser)]
pub struct AutomaticGenerateKeysCmd {
    /// Substrate node RPC URL
    #[arg(long, default_value = "ws://localhost:9944")]
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

    // Step 1: Create subxt client
    let client = OnlineClient::<SubstrateConfig>::from_url(&config.node_url)
        .await
        .context("Failed to create subxt client")?;

    // Step 2: Make the author_rotateKeys RPC call
    let keys_hex: String = RpcClient::from_url(&config.node_url)
        .await
        .context("Failed to create RPC client")?
        .request("author_rotateKeys", RpcParams::new())
        .await
        .context("Failed to call author_rotateKeys RPC method")?;

    context.eprint(&format!("✅ Generated session keys: {}", keys_hex));

    // Step 3: Decode the session keys using the runtime API
    context.eprint("🔍 Decoding session keys...");
    let session_keys = decode_session_keys(&client, &keys_hex, context)
        .await
        .context("Failed to decode session keys")?;

    context.eprint(&format!("✅ Successfully decoded {} session keys", session_keys.len()));

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
        context.eprint("Refusing to overwrite session_keys file - skipping save");
        context.eprint("🔑 Generated session keys:");
        context.print(&json_output);
    }

    Ok(())
}

/// Decode session keys using a dynamic SessionKeys::decode_session_keys runtime call
async fn decode_session_keys<C: IOContext>(
    client: &OnlineClient<SubstrateConfig>,
    keys_hex: &str,
    context: &C,
) -> Result<Vec<SessionKeyInfo>> {
    // Remove 0x prefix if present and decode hex to bytes
    let hex_data = keys_hex.strip_prefix("0x").unwrap_or(keys_hex);
    let encoded_keys = hex::decode(hex_data).context("Failed to decode hex string")?;

    // Dynamic runtime call to SessionKeys::decode_session_keys
    let call = subxt::dynamic::runtime_api_call("SessionKeys", "decode_session_keys", vec![Value::from_bytes(encoded_keys)]);
    let decoded_keys = client
        .runtime_api()
        .at_latest()
        .await
        .context("Failed to get latest block")?
        .call(call)
        .await
        .context("Failed to call decode_session_keys")?
        .to_value()
        .context("Failed to convert to value")?;

    // Debug the decoded keys structure
    context.eprint(&format!("DEBUG: Decoded keys structure: {:?}", decoded_keys));

    // Process the decoded keys
    let session_keys = match decoded_keys.as_vec() {
        Some((_variant_name, value)) => {
            // Expecting a 'Some' variant containing a Vec of tuples
            match value.as_vec() {
                Some(sequence) => {
                    let mut session_keys = Vec::new();
                    for value in sequence {
                        let tuple = value.as_tuple().ok_or(anyhow::anyhow!("Invalid key tuple format"))?;
                        if tuple.len() != 2 {
                            return Err(anyhow::anyhow!("Invalid key tuple length"));
                        }
                        let public_key = tuple[0].as_bytes().ok_or(anyhow::anyhow!("Invalid public key format"))?;
                        let key_type_id = tuple[1].as_bytes().ok_or(anyhow::anyhow!("Invalid key type ID format"))?;
                        let key_type = String::from_utf8(key_type_id.to_vec())
                            .map_err(|e| anyhow::anyhow!("Invalid key type ID: {}", e))?;
                        let public_key_hex = format!("0x{}", hex::encode(&public_key));

                        context.eprint(&format!("  📝 Decoded {} key: {}", key_type, public_key_hex));

                        session_keys.push(SessionKeyInfo {
                            key_type,
                            public_key: public_key_hex,
                        });
                    }
                    session_keys
                }
                None => {
                    context.eprint("  ⚠️  Expected a sequence in Some variant, got different type");
                    vec![SessionKeyInfo {
                        key_type: "raw".to_string(),
                        public_key: keys_hex.to_string(),
                    }]
                }
            }
        }
        None => {
            context.eprint("  ⚠️  Could not decode session keys - got None variant");
            vec![SessionKeyInfo {
                key_type: "raw".to_string(),
                public_key: keys_hex.to_string(),
            }]
        }
    };

    Ok(session_keys)
}
