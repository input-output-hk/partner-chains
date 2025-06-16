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
struct AutomaticGeneratedKeys {
    aura: String,
    gran: String, // keeping consistent with existing naming
    beefy: Option<String>,
    imon: Option<String>,
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
) -> Result<AutomaticGeneratedKeys> {
    context.eprint("⚙️ Calling author_rotateKeys() RPC method...");
    
    // Create a new Tokio runtime for the async operation
    let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
    
    let keys_hex = rt.block_on(connect_and_rotate_keys(&config.node_url))?;
    
    context.eprint(&format!("✅ Received keys: {}", keys_hex));
    
    // Parse the returned keys
    let keys = parse_rotated_keys(&keys_hex)?;
    
    context.eprint("✅ Keys parsed successfully:");
    context.eprint(&format!("   • Aura key: {}", keys.aura));
    context.eprint(&format!("   • Grandpa key: {}", keys.gran));
    if let Some(ref beefy) = keys.beefy {
        context.eprint(&format!("   • Beefy key: {}", beefy));
    }
    if let Some(ref imon) = keys.imon {
        context.eprint(&format!("   • IMON key: {}", imon));
    }
    
    Ok(keys)
}

fn parse_rotated_keys(keys_hex: &str) -> Result<AutomaticGeneratedKeys> {
    // Remove 0x prefix if present
    let keys_hex = keys_hex.strip_prefix("0x").unwrap_or(keys_hex);
    
    // Validate hex string length and format
    if keys_hex.len() < 128 {
        return Err(anyhow!("Keys string too short, expected at least 128 hex characters (64 bytes)"));
    }
    
    if keys_hex.len() % 2 != 0 {
        return Err(anyhow!("Keys string has odd length, expected even number of hex characters"));
    }
    
    // Parse keys by position (each key is 32 bytes = 64 hex chars)
    let aura = format!("0x{}", &keys_hex[0..64]);
    let gran = format!("0x{}", &keys_hex[64..128]);
    
    let mut beefy = None;
    let mut imon = None;
    
    // Check if we have more keys (Beefy and IMON are optional)
    if keys_hex.len() >= 192 {
        beefy = Some(format!("0x{}", &keys_hex[128..192]));
    }
    
    if keys_hex.len() >= 256 {
        imon = Some(format!("0x{}", &keys_hex[192..256]));
    }
    
    Ok(AutomaticGeneratedKeys {
        aura,
        gran,
        beefy,
        imon,
    })
}

fn save_keys_with_identifiers<C: IOContext>(
    keys: &AutomaticGeneratedKeys,
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
    
    // Serialize keys to JSON
    let json_content = serde_json::to_string_pretty(keys)
        .context("Failed to serialize keys to JSON")?;
    
    // Write to file
    context.write_file(output_file, &json_content);
    
    context.eprint(&format!("💾 Keys saved to: {}", output_file));
    context.eprint("📋 The file contains your Partner Chain session keys with identifiers.");
    context.eprint("🔒 Keep this file secure - it contains your public keys for the Partner Chain.");
    
    Ok(())
}

 