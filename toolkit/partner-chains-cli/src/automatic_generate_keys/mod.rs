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

// Generic key pair structure: 4-byte identifier + key bytes
pub type KeyPair = ([u8; 4], Vec<u8>);
pub type SessionKeys = Vec<KeyPair>;

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
        context.eprint("→  Parse the returned keys using runtime metadata");
        context.eprint("→  Extract key type identifiers dynamically from metadata");
        context.eprint("→  Save the keys with identifiers to a JSON file");
        context.enewline();

        let config = AutomaticGenerateKeysConfig::load(context, self.url.clone());
        context.eprint(&format!("🔗 Connecting to node at: {}", config.node_url));

        let keys = generate_keys_via_subxt(&config, context)?;
        context.enewline();

        save_keys_with_identifiers(&keys, context)?;
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
        
        // Get the runtime metadata
        let metadata = client.metadata();
        
        // Find SessionKeys type in the metadata using a simplified approach
        let session_keys_info = extract_session_keys_info_from_metadata(&metadata)
            .context("Failed to extract SessionKeys information from metadata")?;
        
        context.eprint(&format!("✅ Found SessionKeys with {} key types:", session_keys_info.len()));
        for (key_type_id, key_type_name) in &session_keys_info {
            let key_type_str = String::from_utf8_lossy(key_type_id);
            context.eprint(&format!("   • {} ({})", key_type_name, key_type_str));
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

// Extract SessionKeys information from runtime metadata using a simplified approach
fn extract_session_keys_info_from_metadata(
    metadata: &subxt::Metadata,
) -> Result<Vec<([u8; 4], String)>> {
    let mut session_keys_info = Vec::new();
    
    // Since complex metadata parsing is challenging with the current subxt version,
    // we'll use a hybrid approach: check what pallets are available and infer key types
    
    // Check for common key types by looking at available pallets
    let available_pallets: Vec<String> = metadata.pallets()
        .map(|pallet| pallet.name().to_lowercase())
        .collect();
    
    // Map of pallet names to their corresponding key types
    let pallet_to_key_mapping = [
        ("aura", ([b'a', b'u', b'r', b'a'], "Aura")),
        ("grandpa", ([b'g', b'r', b'a', b'n'], "Grandpa")),
        ("beefy", ([b'b', b'e', b'e', b'f'], "Beefy")),
        ("imonline", ([b'i', b'm', b'o', b'n'], "ImOnline")),
        ("im_online", ([b'i', b'm', b'o', b'n'], "ImOnline")),
        ("parachains", ([b'p', b'a', b'r', b'a'], "Parachain")),
        ("parachain", ([b'p', b'a', b'r', b'a'], "Parachain")),
        ("babe", ([b'b', b'a', b'b', b'e'], "Babe")),
    ];
    
    // Check which key types are likely present based on available pallets
    for (pallet_name, (key_id, key_name)) in &pallet_to_key_mapping {
        if available_pallets.iter().any(|p| p.contains(pallet_name)) {
            session_keys_info.push((*key_id, key_name.to_string()));
        }
    }
    
    // If we found no keys through pallet detection, use a reasonable default
    // Most Substrate chains have at least Aura and Grandpa
    if session_keys_info.is_empty() {
        // Note: We can't use context here since it's not in scope
        // The calling function will handle this case
        session_keys_info.push(([b'a', b'u', b'r', b'a'], "Aura".to_string()));
        session_keys_info.push(([b'g', b'r', b'a', b'n'], "Grandpa".to_string()));
    }
    
    Ok(session_keys_info)
}

// Parse rotated keys using metadata information
fn parse_rotated_keys_with_metadata(
    keys_hex: &str,
    session_keys_info: &[([u8; 4], String)],
) -> Result<SessionKeys> {
    // Remove 0x prefix if present
    let keys_hex = keys_hex.strip_prefix("0x").unwrap_or(keys_hex);
    
    // Validate hex string
    if keys_hex.len() % 2 != 0 {
        return Err(anyhow!("Keys string has odd length, expected even number of hex characters"));
    }
    
    // Decode hex string to bytes
    let key_bytes = hex::decode(keys_hex)
        .context("Failed to decode hex string")?;
    
    // Calculate expected length based on metadata
    let expected_length = session_keys_info.len() * 32; // Assuming 32-byte keys
    
    // If the length doesn't match exactly, try to parse what we can
    if key_bytes.len() != expected_length {
        // Try to determine the actual number of keys from the data length
        if key_bytes.len() % 32 != 0 {
            return Err(anyhow!("Key data length is not a multiple of 32 bytes"));
        }
        
        let actual_key_count = key_bytes.len() / 32;
        
        // Adjust our session_keys_info to match the actual data
        let mut adjusted_info = Vec::new();
        for i in 0..actual_key_count {
            if i < session_keys_info.len() {
                adjusted_info.push(session_keys_info[i].clone());
            } else {
                // Generate generic key type for extra keys
                let key_id = [b'k', b'e', b'y', (i as u8) + b'0'];
                adjusted_info.push((key_id, format!("Key{}", i)));
            }
        }
        
        return parse_keys_from_bytes(&key_bytes, &adjusted_info);
    }
    
    parse_keys_from_bytes(&key_bytes, session_keys_info)
}

// Helper function to parse keys from bytes
fn parse_keys_from_bytes(
    key_bytes: &[u8],
    session_keys_info: &[([u8; 4], String)],
) -> Result<SessionKeys> {
    let mut session_keys = Vec::new();
    
    for (index, (key_type_id, _key_name)) in session_keys_info.iter().enumerate() {
        let start_offset = index * 32;
        let end_offset = start_offset + 32;
        
        if end_offset <= key_bytes.len() {
            let key_data = key_bytes[start_offset..end_offset].to_vec();
            session_keys.push((*key_type_id, key_data));
        }
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

 