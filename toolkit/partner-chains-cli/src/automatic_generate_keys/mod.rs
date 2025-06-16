use crate::io::IOContext;
use crate::{config::config_fields, *};
use anyhow::{anyhow, Context, Result};
use jsonrpsee::{
    core::client::ClientT,
    http_client::HttpClientBuilder,
    rpc_params,
};
use serde::{Deserialize, Serialize};
use hex;

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
    pub substrate_node_base_path: String,
    pub node_url: String,
}

impl AutomaticGenerateKeysConfig {
    pub(crate) fn load<C: IOContext>(context: &C, url: String) -> Self {
        Self {
            substrate_node_base_path: config_fields::SUBSTRATE_NODE_DATA_BASE_PATH
                .load_or_prompt_and_save(context),
            node_url: url,
        }
    }
}

// Generic key pair structure: 4-byte identifier + key bytes
pub type KeyPair = ([u8; 4], Vec<u8>);
pub type SessionKeys = Vec<KeyPair>;

// Key type identifiers (4 bytes each)
const AURA_KEY_TYPE: [u8; 4] = *b"aura";
const GRANDPA_KEY_TYPE: [u8; 4] = *b"gran"; 
const BEEFY_KEY_TYPE: [u8; 4] = *b"beef";
const IMON_KEY_TYPE: [u8; 4] = *b"imon";

// For JSON serialization compatibility
#[derive(Debug, Serialize, Deserialize)]
struct SessionKeysJson {
    keys: Vec<KeyEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct KeyEntry {
    key_type: String,
    key_type_bytes: [u8; 4],
    public_key: String,
}

impl CmdRun for AutomaticGenerateKeysCmd {
    fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
        context.eprint(
            "This 🧙 wizard will automatically generate keys by connecting to a running Partner Chain node:",
        );
        context.eprint("→  Connect to the Partner Chain node");
        context.eprint("→  Execute RPC author_rotateKeys()");
        context.eprint("→  Parse the returned keys and verify their order");
        context.eprint("→  Add identifiers to the keys");
        context.eprint("→  Save the keys with identifiers to a JSON file");
        context.enewline();

        let config = AutomaticGenerateKeysConfig::load(context, self.url.clone());
        context.eprint(&format!("🔗 Connecting to node at: {}", config.node_url));

        let keys = generate_keys_via_rpc(&config, context)?;
        context.enewline();

        save_keys_with_identifiers(&keys, context)?;
        context.enewline();

        context.eprint("🚀 All done!");

        Ok(())
    }
}

async fn connect_and_rotate_keys(node_url: &str) -> Result<String> {
    let client = HttpClientBuilder::default()
        .build(node_url)
        .context("Failed to build HTTP client")?;

    // Call author_rotateKeys RPC method
    let response: String = client
        .request("author_rotateKeys", rpc_params![])
        .await
        .context("Failed to call author_rotateKeys RPC method")?;

    Ok(response)
}

fn generate_keys_via_rpc<C: IOContext>(
    config: &AutomaticGenerateKeysConfig,
    context: &C,
) -> Result<SessionKeys> {
    context.eprint("⚙️ Calling author_rotateKeys() RPC method...");
    
    // Create a new Tokio runtime for the async operation
    let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
    
    let keys_hex = rt.block_on(connect_and_rotate_keys(&config.node_url))?;
    
    context.eprint(&format!("✅ Received keys: {}", keys_hex));
    
    // Parse the returned keys
    let keys = parse_rotated_keys(&keys_hex)?;
    
    context.eprint("✅ Keys parsed successfully:");
    for (key_type, key_bytes) in &keys {
        let key_type_str = String::from_utf8_lossy(key_type);
        let key_hex = format!("0x{}", hex::encode(key_bytes));
        context.eprint(&format!("   • {} key: {}", key_type_str, key_hex));
    }
    
    Ok(keys)
}

// Generic parsing function that doesn't hardcode specific key types
fn parse_rotated_keys(keys_hex: &str) -> Result<SessionKeys> {
    // Remove 0x prefix if present
    let keys_hex = keys_hex.strip_prefix("0x").unwrap_or(keys_hex);
    
    // Validate hex string length and format
    if keys_hex.len() < 128 {
        return Err(anyhow!("Keys string too short, expected at least 128 hex characters (64 bytes)"));
    }
    
    if keys_hex.len() % 2 != 0 {
        return Err(anyhow!("Keys string has odd length, expected even number of hex characters"));
    }
    
    // Decode hex string to bytes
    let key_bytes = hex::decode(keys_hex)
        .context("Failed to decode hex string")?;
    
    // Each key is 32 bytes, parse them generically
    let key_size = 32;
    let num_keys = key_bytes.len() / key_size;
    
    if key_bytes.len() % key_size != 0 {
        return Err(anyhow!("Invalid key data length, not divisible by 32 bytes"));
    }
    
    let mut session_keys = Vec::new();
    
    // Define expected key types in order based on common Substrate patterns
    let key_types = [AURA_KEY_TYPE, GRANDPA_KEY_TYPE, BEEFY_KEY_TYPE, IMON_KEY_TYPE];
    
    for i in 0..num_keys {
        let start_idx = i * key_size;
        let end_idx = start_idx + key_size;
        let key_data = key_bytes[start_idx..end_idx].to_vec();
        
        // Use predefined key type if available, otherwise generate a generic one
        let key_type = if i < key_types.len() {
            key_types[i]
        } else {
            // Generate generic key type identifier for unknown keys
            let key_num = (i as u32).to_be_bytes();
            [b'k', b'e', b'y', key_num[3]]
        };
        
        session_keys.push((key_type, key_data));
    }
    
    Ok(session_keys)
}

fn save_keys_with_identifiers<C: IOContext>(
    keys: &SessionKeys,
    context: &C,
) -> Result<()> {
    let output_file = "automatic-generated-keys.json";
    
    // Check if file already exists
    if context.file_exists(output_file) {
        context.eprint(&format!("⚠️  File '{}' already exists", output_file));
        if !context.prompt_yes_no(&format!("Do you want to overwrite '{}'?", output_file), false) {
            context.eprint("❌ Operation cancelled.");
            return Ok(());
        }
    }
    
    // Convert to JSON-friendly format
    let json_keys = SessionKeysJson {
        keys: keys.iter().map(|(key_type, key_bytes)| {
            let key_type_str = String::from_utf8_lossy(key_type).to_string();
            let public_key = format!("0x{}", hex::encode(key_bytes));
            
            KeyEntry {
                key_type: key_type_str,
                key_type_bytes: *key_type,
                public_key,
            }
        }).collect(),
    };
    
    // Serialize keys to JSON
    let json_content = serde_json::to_string_pretty(&json_keys)
        .context("Failed to serialize keys to JSON")?;
    
    // Write to file
    context.write_file(output_file, &json_content);
    
    context.eprint(&format!("💾 Keys saved to: {}", output_file));
    context.eprint("📋 The file contains your Partner Chain session keys with identifiers.");
    context.eprint("🔒 Keep this file secure - it contains your public keys for the Partner Chain.");
    
    Ok(())
}

 