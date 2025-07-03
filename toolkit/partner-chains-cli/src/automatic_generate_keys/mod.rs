use reqwest::Client;
use serde::{Deserialize, Serialize};
use parity_scale_codec::{Decode, Encode};
use std::collections::HashMap;

// JSON-RPC request structure.
#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

// JSON-RPC response structure.
#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// Helper to send a JSON-RPC request.
async fn send_rpc_request<T: for<'de> Deserialize<'de>>(
    client: &Client,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<T, Box<dyn std::error::Error>> {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params,
        id: 1,
    };

    let response = client
        .post(url)
        .json(&request)
        .send()
        .await?
        .json::<JsonRpcResponse<T>>()
        .await?;

    if let Some(error) = response.error {
        return Err(format!("RPC error: {} (code: {})", error.message, error.code).into());
    }

    response.result.ok_or_else(|| "No result in response".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let url = "http://localhost:9933"; // Adjust to your node's RPC endpoint.

    // Step 1: Call author_rotateKeys RPC.
    let session_keys_hex: String = send_rpc_request(
        &client,
        url,
        "author_rotateKeys",
        serde_json::json!([]), // No params.
    )
    .await?;
    println!("Raw session keys (hex): {}", session_keys_hex);

    // Decode hex string to bytes (remove "0x" prefix).
    let session_keys = hex::decode(&session_keys_hex[2..])?;

    // Step 2: Call Session_decodeSessionKeys runtime API via state_call.
    // Get the latest block hash for the runtime call.
    let block_hash: String = send_rpc_request(
        &client,
        url,
        "chain_getFinalizedHead",
        serde_json::json!([]),
    )
    .await?;

    // Prepare the SCALE-encoded session keys as a hex string.
    let session_keys_param = format!("0x{}", hex::encode(&session_keys));
    let params = serde_json::json!([ "Session_decodeSessionKeys", session_keys_param, block_hash ]);

    let decoded_keys_hex: String = send_rpc_request(&client, url, "state_call", params).await?;
    let decoded_keys_bytes = hex::decode(&decoded_keys_hex[2..])?;

    // Step 3: Decode the SCALE-encoded result into Vec<(Vec<u8>, Vec<u8>)>.
    let decoded_keys: Vec<(Vec<u8>, Vec<u8>)> = Decode::decode(&mut &decoded_keys_bytes[..])?;

    // Step 4: Convert to a dictionary.
    let mut key_map: HashMap<String, String> = HashMap::new();
    for (key_type, public_key) in decoded_keys {
        // Convert 4-byte key_type to string (e.g., b"aura" -> "aura").
        let key_type_str = String::from_utf8(key_type)
            .map_err(|e| format!("Invalid key type encoding: {}", e))?;
        // Convert public key to hex string.
        let public_key_hex = format!("0x{}", hex::encode(&public_key));
        key_map.insert(key_type_str, public_key_hex);
    }

    // Print the resulting dictionary.
    println!("Decoded session keys: {:?}", key_map);

    Ok(())
}
