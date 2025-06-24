use crate::io::IOContext;
use crate::*;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

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
        Self {
            node_url: url,
        }
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
        let node_executable = context.current_executable()?;

        generate_keys_via_rpc(&config, &node_executable, context)
    }
}

fn generate_keys_via_rpc<C: IOContext>(
    config: &AutomaticGenerateKeysConfig,
    node_executable: &str,
    context: &C,
) -> Result<()> {
    context.eprint("🔑 Generating session keys via RPC...");
    
    // Step 1: Generate session keys
    let keys_hex = context
        .run_command(&format!(
            r#"{node_executable} rpc author_rotateKeys --url {}"#,
            config.node_url
        ))?
        .trim()
        .trim_matches('"')
        .to_string();

    context.eprint(&format!("✅ Generated session keys: {}", keys_hex));
    
    // Step 2: Decode session keys to get actual key types
    context.eprint("🔍 Decoding session keys to get key types...");
    
    let decoded_keys = decode_session_keys_via_rpc(&keys_hex, node_executable, &config.node_url, context)
        .context("Failed to decode session keys")?;

    // Step 3: Save to JSON file
    let output_path = "session_keys.json";
    let json_output = serde_json::to_string_pretty(&decoded_keys)
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

/// Decode session keys using the node's sessionKeys_decodeSessionKeys RPC call
fn decode_session_keys_via_rpc<C: IOContext>(
    keys_hex: &str,
    node_executable: &str,
    node_url: &str,
    context: &C,
) -> Result<Vec<SessionKeyInfo>> {
    // Call sessionKeys_decodeSessionKeys RPC
    let decode_result = context
        .run_command(&format!(
            r#"{node_executable} rpc sessionKeys_decodeSessionKeys --params '["{}"]' --url {}"#,
            keys_hex, node_url
        ))?
        .trim()
        .to_string();

    context.eprint(&format!("✅ Decode response: {}", decode_result));

    // Parse the JSON response
    let json_value: serde_json::Value = serde_json::from_str(&decode_result)
        .context("Failed to parse decode response as JSON")?;

    // Extract the array from the response
    let key_array = json_value
        .as_array()
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
    Ok(session_keys)
}
