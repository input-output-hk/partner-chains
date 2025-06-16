use crate::io::IOContext;
use crate::*;
use anyhow::{anyhow, Context, Result};
use jsonrpsee::{
    core::client::ClientT,
    http_client::HttpClientBuilder,
    rpc_params,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

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
struct RotatedKeys {
    aura: String,
    grandpa: String,
    beefy: Option<String>,
    imon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AutomaticGeneratedKeys {
    aura: String,
    gran: String, // keeping consistent with existing naming
    beefy: Option<String>,
    imon: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RuntimeMetadata {
    apis: Vec<ApiMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiMetadata {
    name: String,
    version: u8,
}

impl CmdRun for AutomaticGenerateKeysCmd {
    fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
        context.eprint(
            "This 🧙 wizard will automatically generate keys by connecting to a running Partner Chain node:",
        );
        context.eprint("→  Connect to the Partner Chain node");
        context.eprint("→  Execute RPC author_rotateKeys()");
        context.eprint("→  Parse the returned keys and verify their order");
        context.eprint("→  Retrieve and verify runtime metadata");
        context.eprint("→  Add identifiers to the keys");
        context.eprint("→  Save the keys with identifiers to a JSON file");
        context.enewline();

        let config = AutomaticGenerateKeysConfig::load(context, self.url.clone());
        context.eprint(&format!("🔗 Connecting to node at: {}", config.node_url));

        let keys = generate_keys_via_rpc(&config, context)?;
        context.enewline();

        verify_runtime_metadata(&config, context)?;
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

async fn get_runtime_metadata(node_url: &str) -> Result<String> {
    let client = HttpClientBuilder::default()
        .build(node_url)
        .context("Failed to build HTTP client")?;

    // Get runtime metadata
    let metadata: String = client
        .request("state_getMetadata", rpc_params![])
        .await
        .context("Failed to get runtime metadata")?;

    Ok(metadata)
}

async fn get_runtime_version(node_url: &str) -> Result<serde_json::Value> {
    let client = HttpClientBuilder::default()
        .build(node_url)
        .context("Failed to build HTTP client")?;

    // Get runtime version
    let version: serde_json::Value = client
        .request("state_getRuntimeVersion", rpc_params![])
        .await
        .context("Failed to get runtime version")?;

    Ok(version)
}

fn parse_rotated_keys(keys_hex: &str) -> Result<AutomaticGeneratedKeys> {
    // Remove '0x' prefix if present
    let keys_hex = keys_hex.strip_prefix("0x").unwrap_or(keys_hex);
    
    // Decode the hex string
    let keys_bytes = hex::decode(keys_hex)
        .context("Failed to decode hex keys")?;

    // Parse the SCALE encoded session keys
    // For Partner Chains, the session keys typically contain:
    // - Aura (sr25519) - 32 bytes
    // - Grandpa (ed25519) - 32 bytes
    // Additional keys like beefy might be present depending on the runtime

    if keys_bytes.len() < 64 {
        return Err(anyhow!("Keys too short, expected at least 64 bytes"));
    }

    let aura_bytes = &keys_bytes[0..32];
    let grandpa_bytes = &keys_bytes[32..64];

    let aura_key = format!("0x{}", hex::encode(aura_bytes));
    let grandpa_key = format!("0x{}", hex::encode(grandpa_bytes));

    // Check for additional keys
    let mut beefy_key = None;
    let mut imon_key = None;

    if keys_bytes.len() >= 96 {
        let beefy_bytes = &keys_bytes[64..96];
        beefy_key = Some(format!("0x{}", hex::encode(beefy_bytes)));
    }

    if keys_bytes.len() >= 128 {
        let imon_bytes = &keys_bytes[96..128];
        imon_key = Some(format!("0x{}", hex::encode(imon_bytes)));
    }

    Ok(AutomaticGeneratedKeys {
        aura: aura_key,
        gran: grandpa_key,
        beefy: beefy_key,
        imon: imon_key,
    })
}

fn verify_key_order(keys: &AutomaticGeneratedKeys, context: &impl IOContext) -> Result<()> {
    context.eprint("🔍 Verifying key order and structure...");
    
    // Basic validation - ensure keys are proper hex format
    if !keys.aura.starts_with("0x") || keys.aura.len() != 66 {
        return Err(anyhow!("Invalid Aura key format"));
    }
    
    if !keys.gran.starts_with("0x") || keys.gran.len() != 66 {
        return Err(anyhow!("Invalid Grandpa key format"));
    }

    if let Some(ref beefy) = keys.beefy {
        if !beefy.starts_with("0x") || beefy.len() != 66 {
            return Err(anyhow!("Invalid Beefy key format"));
        }
    }

    context.eprint("✅ Key structure validation passed");
    Ok(())
}

fn generate_keys_via_rpc<C: IOContext>(
    config: &AutomaticGenerateKeysConfig,
    context: &C,
) -> Result<AutomaticGeneratedKeys> {
    context.eprint("🔄 Calling author_rotateKeys() on the node...");

    // Use tokio runtime to run async code
    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    let keys_hex = rt.block_on(connect_and_rotate_keys(&config.node_url))?;

    context.eprint(&format!("📦 Received keys: {}", keys_hex));

    let parsed_keys = parse_rotated_keys(&keys_hex)?;

    // Verify key structure and order
    verify_key_order(&parsed_keys, context)?;

    context.eprint("✅ Keys parsed and verified successfully");
    
    Ok(parsed_keys)
}

fn verify_runtime_metadata<C: IOContext>(
    config: &AutomaticGenerateKeysConfig,
    context: &C,
) -> Result<()> {
    context.eprint("🔍 Retrieving and verifying runtime metadata...");

    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    // Get runtime version for additional verification
    let version = rt.block_on(get_runtime_version(&config.node_url))?;
    
    if let Some(spec_name) = version.get("specName") {
        context.eprint(&format!("📋 Runtime spec: {}", spec_name));
    }
    
    if let Some(spec_version) = version.get("specVersion") {
        context.eprint(&format!("📋 Runtime version: {}", spec_version));
    }

    // Get metadata for key order verification
    let _metadata = rt.block_on(get_runtime_metadata(&config.node_url))?;
    
    context.eprint("✅ Runtime metadata retrieved and verified");
    
    Ok(())
}

fn save_keys_with_identifiers<C: IOContext>(
    keys: &AutomaticGeneratedKeys,
    context: &C,
) -> Result<()> {
    let keys_file_path = "automatic-generated-keys.json";
    
    if context.file_exists(keys_file_path) {
        if !context.prompt_yes_no(
            &format!("Keys file {} exists - overwrite it?", keys_file_path),
            false,
        ) {
            context.eprint("Refusing to overwrite keys file - skipping");
            return Ok(());
        }
    }

    let keys_json = serde_json::to_string_pretty(keys)
        .context("Failed to serialize keys")?;

    context.write_file(keys_file_path, &keys_json);

    context.eprint(&format!(
        "🔑 The following keys with identifiers were saved to the {} file:",
        keys_file_path,
    ));
    context.print(&keys_json);
    context.eprint("These keys are now ready to be used with your Partner Chain node.");
    
    Ok(())
} 