use crate::io::IOContext;
use crate::*;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use subxt::{OnlineClient, PolkadotConfig};

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
    pub node_url: String,
}

impl AutomaticGenerateKeysConfig {
    pub(crate) fn load<C: IOContext>(_context: &C, url: String) -> Self {
        Self {
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
    pub public_key: String,
}

#[derive(Debug)]
struct SessionKeyTypeInfo {
    key_type_id: [u8; 4],
    key_name: String,
    key_size: usize,
}

impl CmdRun for AutomaticGenerateKeysCmd {
    fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
        context.print("🔐 Generating session keys automatically using runtime metadata...");
        context.enewline();

        let config = AutomaticGenerateKeysConfig::load(context, self.url.clone());
        context.eprint(&format!("🔗 Connecting to node at: {}", config.node_url));

        let keys = generate_keys_via_runtime(&config, context)?;
        context.enewline();

        // Convert to output format and print as JSON
        let session_keys_info: Vec<SessionKeyInfo> = keys
            .iter()
            .map(|(key_type_id, public_key)| {
                let key_type_str = String::from_utf8_lossy(key_type_id).to_string();
                SessionKeyInfo {
                    key_type: key_type_str.clone(),
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

fn generate_keys_via_runtime<C: IOContext>(
    config: &AutomaticGenerateKeysConfig,
    context: &C,
) -> Result<SessionKeys> {
    context.eprint("⚙️ Connecting to node and analyzing runtime...");
    
    // Create a new Tokio runtime for the async operation
    let rt = tokio::runtime::Runtime::new().context("Failed to create Tokio runtime")?;
    
    let keys = rt.block_on(async {
        // Connect to the node using subxt to get metadata
        let client = OnlineClient::<PolkadotConfig>::from_url(&config.node_url)
            .await
            .context("Failed to connect to node")?;
        
        context.eprint("✅ Connected to node successfully");
        context.eprint("🔍 Extracting SessionKeys type definition from runtime metadata...");
        
        // Extract the actual SessionKeys type definition from metadata
        let metadata = client.metadata();
        let session_keys_type_info = extract_session_keys_type_from_metadata(&metadata)
            .context("Failed to extract SessionKeys type from runtime metadata")?;
        
        context.eprint(&format!("📋 Found SessionKeys with {} key types:", session_keys_type_info.len()));
        for key_info in &session_keys_type_info {
            let key_id_str = String::from_utf8_lossy(&key_info.key_type_id);
            context.eprint(&format!("   - {} ({}, {} bytes)", key_info.key_name, key_id_str, key_info.key_size));
        }
        
        // Step 1: Generate keys using author_rotateKeys RPC via node executable
        context.eprint("⚙️ Calling author_rotateKeys() via node executable...");
        
        let node_executable = context.current_executable()?;
        let keys_hex = context
            .run_command(&format!("{node_executable} rpc author_rotateKeys --url {}", config.node_url))?
            .trim()
            .to_string();
        
        context.eprint(&format!("✅ Generated session keys: {}", keys_hex));
        
        // Step 2: Decode the keys using the extracted type information
        context.eprint("🔍 Decoding session keys using runtime type information...");
        
        let decoded_keys = decode_session_keys_from_type_info(&keys_hex, &session_keys_type_info)
            .context("Failed to decode session keys using type information")?;
        
        context.eprint(&format!("✅ Successfully decoded {} session keys:", decoded_keys.len()));
        for (key_type, key_bytes) in &decoded_keys {
            let key_type_str = String::from_utf8_lossy(key_type);
            let key_hex = format!("0x{}", hex::encode(key_bytes));
            context.eprint(&format!("   • {} key: {}", key_type_str, key_hex));
        }
        
        Ok::<SessionKeys, anyhow::Error>(decoded_keys)
    })?;
    
    Ok(keys)
}

/// Extract the actual SessionKeys type definition from runtime metadata
/// This is truly runtime-aware - it reads the actual type definition
fn extract_session_keys_type_from_metadata(
    metadata: &subxt::Metadata,
) -> Result<Vec<SessionKeyTypeInfo>> {
    // Look for the SessionKeys type in the metadata
    let session_keys_type = find_session_keys_type(metadata)
        .context("SessionKeys type not found in runtime metadata")?;
    
    // Parse the SessionKeys type definition
    parse_session_keys_type(session_keys_type, metadata)
}

/// Find the SessionKeys type in the metadata by looking at type paths
fn find_session_keys_type(metadata: &subxt::Metadata) -> Result<&scale_info::Type<scale_info::form::PortableForm>> {
    // Look through all types to find one that looks like SessionKeys
    for (_type_id, portable_type) in metadata.types().types.iter().enumerate() {
        // Check if this type has a path that indicates it's SessionKeys
        let path = &portable_type.ty.path;
        let path_segments: Vec<&str> = path.segments.iter().map(|s| s.as_str()).collect();
        
        // Look for types with SessionKeys in their path
        if path_segments.iter().any(|segment| segment.contains("SessionKeys")) {
            return Ok(&portable_type.ty);
        }
        
        // If no direct "SessionKeys" match found, try runtime-specific patterns
        if path_segments.len() >= 2 {
            let last_two = &path_segments[path_segments.len()-2..];
            let has_runtime = last_two.iter().any(|s| s.to_lowercase().contains("runtime"));
            let has_session_keys = last_two.iter().any(|s| s.to_lowercase().contains("sessionkeys"));
            if has_runtime && has_session_keys {
                return Ok(&portable_type.ty);
            }
        }
    }
    
    Err(anyhow!("SessionKeys type not found in metadata"))
}

/// Parse the SessionKeys type definition to extract key type information
/// This is where the magic happens - we read the actual runtime type definition
fn parse_session_keys_type(
    session_keys_type: &scale_info::Type<scale_info::form::PortableForm>, 
    metadata: &subxt::Metadata
) -> Result<Vec<SessionKeyTypeInfo>> {
    let mut key_types = Vec::new();
    
    match &session_keys_type.type_def {
        scale_info::TypeDef::Composite(composite) => {
            // SessionKeys is typically a composite type with named fields
            for field in &composite.fields {
                if let Some(field_name) = &field.name {
                    // Extract key type information from the field
                    let key_info = extract_key_info_from_field(field_name, field.ty.id, metadata)?;
                    key_types.push(key_info);
                }
            }
        }
        scale_info::TypeDef::Tuple(tuple) => {
            // SessionKeys might be a tuple type
            for (index, field_type) in tuple.fields.iter().enumerate() {
                // Try to infer key type from the field type
                let key_info = extract_key_info_from_type_id(field_type.id, index, metadata)?;
                key_types.push(key_info);
            }
        }
        _ => {
            return Err(anyhow!("Unsupported SessionKeys type definition"));
        }
    }
    
    if key_types.is_empty() {
        return Err(anyhow!("No key types found in SessionKeys definition"));
    }
    
    Ok(key_types)
}

/// Extract key information from a named field
fn extract_key_info_from_field(field_name: &str, type_id: u32, metadata: &subxt::Metadata) -> Result<SessionKeyTypeInfo> {
    // Get the type definition
    let field_type = metadata.types().resolve(type_id)
        .ok_or_else(|| anyhow!("Failed to resolve field type"))?;
    
    // Determine key size from the type
    let key_size = determine_key_size_from_type_with_metadata(field_type, metadata)?;
    
    // Generate key type ID from field name
    let key_type_id = generate_key_type_id_from_name(field_name);
    
    Ok(SessionKeyTypeInfo {
        key_type_id,
        key_name: field_name.to_string(),
        key_size,
    })
}

/// Extract key information from a type ID (for tuple types)
fn extract_key_info_from_type_id(type_id: u32, _index: usize, metadata: &subxt::Metadata) -> Result<SessionKeyTypeInfo> {
    let field_type = metadata.types().resolve(type_id)
        .ok_or_else(|| anyhow!("Failed to resolve field type"))?;
    
    // Determine key size from the type
    let key_size = determine_key_size_from_type_with_metadata(field_type, metadata)?;
    
    // Try to infer key type from the type path
    let key_name = extract_key_name_from_path(&field_type.path);
    
    let key_type_id = generate_key_type_id_from_name(&key_name);
    
    Ok(SessionKeyTypeInfo {
        key_type_id,
        key_name,
        key_size,
    })
}

/// Determine the size of a key from its type definition
/// This reads the actual type definition from metadata and recursively resolves nested types
fn determine_key_size_from_type_with_metadata(
    ty: &scale_info::Type<scale_info::form::PortableForm>, 
    metadata: &subxt::Metadata
) -> Result<usize> {
    determine_key_size_from_type_recursive(ty, metadata, &mut std::collections::HashSet::new())
}

/// Recursive helper to determine key size, with cycle detection
fn determine_key_size_from_type_recursive(
    ty: &scale_info::Type<scale_info::form::PortableForm>,
    metadata: &subxt::Metadata,
    visited: &mut std::collections::HashSet<u32>
) -> Result<usize> {
    match &ty.type_def {
        scale_info::TypeDef::Array(array) => {
            // For array types like [u8; 32], return the length
            Ok(array.len as usize)
        }
        scale_info::TypeDef::Composite(composite) => {
            // For composite types, recurse into the fields to find the actual size
            if composite.fields.len() == 1 {
                // Single field composite - likely a newtype wrapper, recurse into it
                let field = &composite.fields[0];
                
                // Cycle detection - prevent infinite recursion
                if visited.contains(&field.ty.id) {
                    return Err(anyhow!("Circular type reference detected"));
                }
                visited.insert(field.ty.id);
                
                // Resolve the field type and recurse
                let field_type = metadata.types().resolve(field.ty.id)
                    .ok_or_else(|| anyhow!("Failed to resolve field type"))?;
                
                determine_key_size_from_type_recursive(field_type, metadata, visited)
            } else if composite.fields.is_empty() {
                // Empty composite (unit struct)
                Ok(0)
            } else {
                // Multi-field composite - sum the sizes of all fields
                let mut total_size = 0;
                for field in &composite.fields {
                    // Cycle detection
                    if visited.contains(&field.ty.id) {
                        return Err(anyhow!("Circular type reference detected"));
                    }
                    visited.insert(field.ty.id);
                    
                    // Resolve and recurse into each field
                    let field_type = metadata.types().resolve(field.ty.id)
                        .ok_or_else(|| anyhow!("Failed to resolve field type"))?;
                    
                    total_size += determine_key_size_from_type_recursive(field_type, metadata, visited)?;
                }
                Ok(total_size)
            }
        }
        scale_info::TypeDef::Primitive(primitive) => {
            // Handle primitive types
            use scale_info::TypeDefPrimitive;
            match primitive {
                TypeDefPrimitive::Bool => Ok(1),
                TypeDefPrimitive::Char => Ok(4), // UTF-8 char
                TypeDefPrimitive::Str => Err(anyhow!("String types don't have fixed size")),
                TypeDefPrimitive::U8 | TypeDefPrimitive::I8 => Ok(1),
                TypeDefPrimitive::U16 | TypeDefPrimitive::I16 => Ok(2),
                TypeDefPrimitive::U32 | TypeDefPrimitive::I32 => Ok(4),
                TypeDefPrimitive::U64 | TypeDefPrimitive::I64 => Ok(8),
                TypeDefPrimitive::U128 | TypeDefPrimitive::I128 => Ok(16),
                TypeDefPrimitive::U256 | TypeDefPrimitive::I256 => Ok(32),
            }
        }
        scale_info::TypeDef::Sequence(_) => {
            // Sequences (Vec<T>) don't have fixed size
            Err(anyhow!("Sequence types don't have fixed size"))
        }
        scale_info::TypeDef::Tuple(tuple) => {
            // Tuples - sum the sizes of all elements
            if tuple.fields.is_empty() {
                Ok(0) // Unit tuple ()
            } else {
                let mut total_size = 0;
                for field_type in &tuple.fields {
                    // Cycle detection
                    if visited.contains(&field_type.id) {
                        return Err(anyhow!("Circular type reference detected"));
                    }
                    visited.insert(field_type.id);
                    
                    // Resolve and recurse into each tuple element
                    let resolved_type = metadata.types().resolve(field_type.id)
                        .ok_or_else(|| anyhow!("Failed to resolve tuple element type"))?;
                    
                    total_size += determine_key_size_from_type_recursive(resolved_type, metadata, visited)?;
                }
                Ok(total_size)
            }
        }
        scale_info::TypeDef::Compact(_) => {
            // Compact encoding - variable size, not suitable for keys
            Err(anyhow!("Compact types don't have fixed size"))
        }
        scale_info::TypeDef::BitSequence(_) => {
            // Bit sequences - variable size
            Err(anyhow!("Bit sequences don't have fixed size"))
        }
        _ => {
            // For any other types (enums, etc.), default to common key size
            // Most crypto keys in Substrate are 32 bytes
            Ok(32)
        }
    }
}

/// Generate a 4-byte key type ID from a key name
fn generate_key_type_id_from_name(name: &str) -> [u8; 4] {
    let mut key_id = [0u8; 4];
    let name_lower = name.to_lowercase();
    let name_bytes = name_lower.as_bytes();
    
    // Take up to 4 characters from the name
    let len = name_bytes.len().min(4);
    key_id[..len].copy_from_slice(&name_bytes[..len]);
    
    key_id
}

/// Extract key name from a type path
fn extract_key_name_from_path(path: &scale_info::Path<scale_info::form::PortableForm>) -> String {
    if let Some(last_segment) = path.segments.last() {
        // Remove common suffixes and extract the key type
        let segment = last_segment;
        if segment.ends_with("Id") {
            segment.strip_suffix("Id").unwrap_or(segment).to_string()
        } else if segment.ends_with("Public") {
            segment.strip_suffix("Public").unwrap_or(segment).to_string()
        } else {
            segment.to_string()
        }
    } else {
        "Unknown".to_string()
    }
}

/// Decode session keys using the extracted type information
/// This is truly generic - it uses the actual type information from the runtime
fn decode_session_keys_from_type_info(
    keys_hex: &str,
    session_keys_type_info: &[SessionKeyTypeInfo],
) -> Result<SessionKeys> {
    let keys_bytes = hex::decode(keys_hex.strip_prefix("0x").unwrap_or(keys_hex))
        .context("Failed to decode hex string")?;
    
    // Calculate expected total size based on actual type information
    let expected_total_size: usize = session_keys_type_info.iter().map(|info| info.key_size).sum();

    let keys_bytes_length = keys_bytes.len();
    
    if keys_bytes_length != expected_total_size {
        return Err(anyhow!(
            "Invalid session keys length: expected {} bytes, got {}. Difference {}",
            expected_total_size,
            keys_bytes_length,
            (expected_total_size - keys_bytes_length)
        ));
    }
    
    let mut session_keys: SessionKeys = Vec::new();
    let mut offset = 0;
    
    // Decode each key using its actual size from the type definition
    for key_info in session_keys_type_info {
        let key_bytes = keys_bytes[offset..offset + key_info.key_size].to_vec();
        let key_pair: KeyPair = (key_info.key_type_id, key_bytes);
        session_keys.push(key_pair);
        offset += key_info.key_size;
    }
    
    Ok(session_keys)
}
