use crate::io::IOContext;
use crate::{config::config_fields, *};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use subxt::{OnlineClient, PolkadotConfig};
use jsonrpsee::{
    core::client::ClientT,
    http_client::HttpClientBuilder,
    rpc_params,
};
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

// Generic session keys type - list of (key_type_id, public_key) pairs
pub type SessionKeys = Vec<KeyPair>;
pub type KeyPair = ([u8; 4], Vec<u8>);

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionKeyInfo {
    pub key_type: String,
    pub key_type_bytes: String, 
    pub public_key: String,
}

impl CmdRun for AutomaticGenerateKeysCmd {
    fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
        context.eprint(
            "This 🧙 wizard will automatically generate keys by connecting to a running Partner Chain node:",
        );
        context.eprint("→  Connect to the Partner Chain node");
        context.eprint("→  Execute RPC author_rotateKeys()");
        context.eprint("→  Parse the returned keys using runtime metadata");
        context.eprint("→  Extract key type identifiers dynamically from metadata");
        context.eprint("→  Save the keys with identifiers to a JSON file");
        context.enewline();

        let config = AutomaticGenerateKeysConfig::load(context, self.url.clone());
        context.eprint(&format!("🔗 Connecting to node at: {}", config.node_url));

        let keys = generate_keys_via_subxt(&config, context)?;
        context.enewline();

        // Convert to output format and print as JSON
        let session_keys_info: Vec<SessionKeyInfo> = keys
            .iter()
            .map(|(key_type_id, public_key)| {
                let key_type_str = String::from_utf8_lossy(key_type_id).to_string();
                SessionKeyInfo {
                    key_type: key_type_str.clone(),
                    key_type_bytes: hex::encode(key_type_id),
                    public_key: hex::encode(public_key),
                }
            })
            .collect();

        // Output as JSON
        let json_output = serde_json::to_string_pretty(&session_keys_info)
            .context("Failed to serialize session keys to JSON")?;

        context.print(&json_output);
        context.enewline();

        context.eprint("🚀 All done!");

        Ok(())
    }
}

fn generate_keys_via_subxt<C: IOContext>(
    config: &AutomaticGenerateKeysConfig,
    context: &C,
) -> Result<SessionKeys> {
    context.eprint("⚙️ Connecting to node and fetching metadata...");
    
    // Create a new Tokio runtime for the async operation
    let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
    
    let keys = rt.block_on(async {
        // Connect to the node using subxt
        let client = OnlineClient::<PolkadotConfig>::from_url(&config.node_url)
            .await
            .context("Failed to connect to node")?;
        
        context.eprint("✅ Connected to node successfully");
        context.eprint("🔍 Analyzing runtime metadata for SessionKeys structure...");
        
        // Try to get session keys information from metadata
        let session_keys_info = match get_session_keys_from_metadata(&client).await {
            Ok(info) => {
                context.eprint("✅ Successfully extracted session keys info from runtime metadata");
                info
            }
            Err(e) => {
                context.eprint(&format!("⚠️  Could not extract session keys from metadata: {}", e));
                context.eprint("   Using smart fallback based on common Substrate patterns");
                get_default_session_keys_info()
            }
        };
        
        context.eprint(&format!("📋 Detected {} session key types:", session_keys_info.len()));
        for (key_id, key_name) in &session_keys_info {
            let key_id_str = String::from_utf8_lossy(key_id);
            context.eprint(&format!("   - {} ({})", key_name, key_id_str));
        }
        
        context.eprint("⚙️ Calling author_rotateKeys() RPC method...");
        
        // Call author_rotateKeys using jsonrpsee directly
        let rpc_client = HttpClientBuilder::default()
            .build(&config.node_url)
            .context("Failed to build RPC client")?;
            
        let keys_hex: String = rpc_client
            .request("author_rotateKeys", rpc_params![])
            .await
            .context("Failed to call author_rotateKeys RPC method")?;
        
        context.eprint(&format!("✅ Received keys: {}", keys_hex));
        
        // Parse the keys using the metadata information
        let keys = parse_rotated_keys_with_metadata(&keys_hex, &session_keys_info)?;
        
        context.eprint("✅ Keys parsed successfully using runtime metadata:");
        for (key_type, key_bytes) in &keys {
            let key_type_str = String::from_utf8_lossy(key_type);
            let key_hex = format!("0x{}", hex::encode(key_bytes));
            context.eprint(&format!("   • {} key: {}", key_type_str, key_hex));
        }
        
        Ok::<SessionKeys, anyhow::Error>(keys)
    })?;
    
    Ok(keys)
}

/// Attempts to extract session keys information from runtime metadata
async fn get_session_keys_from_metadata(
    client: &OnlineClient<PolkadotConfig>,
) -> Result<Vec<([u8; 4], String)>> {
    let metadata = client.metadata();
    
    // Look for session-related pallets and types
    let mut session_keys_info = Vec::new();
    
    // Check for common session key types by looking for related pallets
    let common_key_types = [
        ("aura", "Aura"),
        ("gran", "Grandpa"), 
        ("beef", "Beefy"),
        ("imon", "ImOnline"),
        ("babe", "Babe"),
        ("auth", "AuthorityDiscovery"),
    ];

    for (key_id_str, key_name) in &common_key_types {
        // Check if this key type might be present by looking for related pallets
        if metadata_contains_key_type(&metadata, key_name) {
            let mut key_id = [0u8; 4];
            let key_bytes = key_id_str.as_bytes();
            key_id[..key_bytes.len().min(4)].copy_from_slice(&key_bytes[..key_bytes.len().min(4)]);
            session_keys_info.push((key_id, key_name.to_string()));
        }
    }

    if session_keys_info.is_empty() {
        return Err(anyhow!("No session key types detected in metadata"));
    }

    Ok(session_keys_info)
}

/// Check if metadata contains references to a specific key type
fn metadata_contains_key_type(metadata: &subxt::Metadata, key_name: &str) -> bool {
    // Look for pallets with names that suggest this key type is used
    let pallet_names = [
        key_name.to_lowercase(),
        format!("pallet_{}", key_name.to_lowercase()),
        key_name.to_uppercase(),
    ];
    
    for pallet_name in &pallet_names {
        if metadata.pallet_by_name(pallet_name).is_some() {
            return true;
        }
    }
    
    false
}

/// Fallback session keys info for common Substrate setups
fn get_default_session_keys_info() -> Vec<([u8; 4], String)> {
    vec![
        ([b'a', b'u', b'r', b'a'], "Aura".to_string()),
        ([b'g', b'r', b'a', b'n'], "Grandpa".to_string()),
    ]
}

/// Parse rotated keys using metadata-provided session keys info
pub fn parse_rotated_keys_with_metadata(
    keys_hex: &str,
    session_keys_info: &[([u8; 4], String)],
) -> Result<SessionKeys> {
    let keys_bytes = hex::decode(keys_hex.strip_prefix("0x").unwrap_or(keys_hex))
        .context("Failed to decode hex string")?;

    decode_session_keys_from_key_info(&keys_bytes, session_keys_info)
}

/// Decode session keys using provided key type information
pub fn decode_session_keys_from_key_info(
    keys_bytes: &[u8],
    session_keys_info: &[([u8; 4], String)],
) -> Result<SessionKeys> {
    let expected_keys = session_keys_info.len();
    
    // Most Substrate session keys are 32 bytes each
    const KEY_SIZE: usize = 32;
    let expected_total_size = expected_keys * KEY_SIZE;
    
    if keys_bytes.len() != expected_total_size {
        return Err(anyhow!(
            "Invalid session keys length: expected {} bytes for {} keys, got {}",
            expected_total_size,
            expected_keys,
            keys_bytes.len()
        ));
    }

    let mut session_keys = Vec::new();
    
    for (i, (key_type_id, _key_name)) in session_keys_info.iter().enumerate() {
        let start = i * KEY_SIZE;
        let end = start + KEY_SIZE;
        let key_bytes = keys_bytes[start..end].to_vec();
        
        session_keys.push((*key_type_id, key_bytes));
    }

    Ok(session_keys)
}



 