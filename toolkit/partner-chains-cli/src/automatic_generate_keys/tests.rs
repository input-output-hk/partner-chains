use super::*;
use crate::tests::{MockIO, MockIOContext};
use mockall::{mock, predicate::*};
use subxt::{OnlineClient, SubstrateConfig, dynamic::Value, rpc::types::RuntimeApiCall};
use tokio;

mock! {
    pub OnlineClient {
        async fn from_url(url: &str) -> Result<Self, subxt::Error>;
        fn runtime_api(&self) -> MockRuntimeApi;
    }
}

mock! {
    pub RpcClient {
        async fn from_url(url: &str) -> Result<Self, subxt::Error>;
        async fn call(&self, method: &str, params: ()) -> Result<String, subxt::Error>;
    }
}

mock! {
    pub RuntimeApi {
        async fn at_latest(&self) -> Result<MockBlockRef, subxt::Error>;
    }
}

mock! {
    pub BlockRef {
        fn call(&self, call: RuntimeApiCall) -> Result<subxt::dynamic::Value, subxt::Error>;
    }
}

#[test]
fn test_config_creation() {
    let mock_context = MockIOContext::new();
    let url = "http://example.com:9944".to_string();

    let config = AutomaticGenerateKeysConfig::load(&mock_context, url);
    assert_eq!(config.node_url, "http://example.com:9944");
}

#[tokio::test]
async fn test_decode_session_keys_with_valid_data() {
    let mock_context = MockIOContext::new().with_expected_io(vec![
        MockIO::eprint("🔍 Decoding session keys..."),
        MockIO::eprint("  📝 Decoded aura key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
        MockIO::eprint("  📝 Decoded gran key: 0x5678901234567890ab5678901234567890ab5678901234567890ab5678901234"),
    ]);

    let keys_hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab5678901234567890ab5678901234";
    let mut mock_client = MockOnlineClient::from_url("ws://localhost:9944").await.unwrap();
    mock_client.expect_runtime_api()
        .once()
        .returning(|| {
            let mut runtime_api = MockRuntimeApi::new();
            runtime_api.expect_at_latest()
                .once()
                .returning(|| {
                    let mut block_ref = MockBlockRef::new();
                    block_ref.expect_call()
                        .once()
                        .with(eq(subxt::dynamic::runtime_api_call(
                            "SessionKeys",
                            "decode_session_keys",
                            vec![Value::from_bytes(hex::decode(keys_hex.strip_prefix("0x").unwrap()).unwrap())],
                        )))
                        .returning(|_| {
                            Ok(Value::unnamed_composite(vec![
                                Value::unnamed_composite(vec![
                                    Value::from_bytes(hex::decode("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap()),
                                    Value::from_bytes(b"aura".to_vec()),
                                ]),
                                Value::unnamed_composite(vec![
                                    Value::from_bytes(hex::decode("5678901234567890ab5678901234567890ab5678901234567890ab5678901234").unwrap()),
                                    Value::from_bytes(b"gran".to_vec()),
                                ]),
                            ]))
                        });
                    Ok(block_ref)
                });
            runtime_api
        });

    let session_keys = decode_session_keys(&mock_client, keys_hex, &mock_context)
        .await
        .expect("Failed to decode session keys");

    assert_eq!(session_keys.len(), 2);
    assert_eq!(session_keys[0].key_type, "aura");
    assert_eq!(session_keys[0].public_key, "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
    assert_eq!(session_keys[1].key_type, "gran");
    assert_eq!(session_keys[1].public_key, "0x5678901234567890ab5678901234567890ab5678901234567890ab5678901234");
}

#[test]
fn test_session_key_info_serialization() {
    let session_key = SessionKeyInfo {
        key_type: "aura".to_string(),
        public_key: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef".to_string(),
    };

    let json = serde_json::to_string(&session_key).unwrap();
    assert!(json.contains("aura"));
    assert!(json.contains("0x1234567890abcdef"));

    let deserialized: SessionKeyInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.key_type, "aura");
    assert_eq!(deserialized.public_key, "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef");
}

#[test]
fn test_hex_decoding() {
    // Test valid hex with 0x prefix
    let hex_with_prefix = "0x1234abcd";
    let stripped = hex_with_prefix.strip_prefix("0x").unwrap_or(hex_with_prefix);
    let decoded = hex::decode(stripped);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap(), vec![0x12, 0x34, 0xab, 0xcd]);

    // Test valid hex without prefix
    let hex_without_prefix = "1234abcd";
    let decoded = hex::decode(hex_without_prefix);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap(), vec![0x12, 0x34, 0xab, 0xcd]);

    // Test invalid hex
    let invalid_hex = "invalid_hex";
    let decoded = hex::decode(invalid_hex);
    assert!(decoded.is_err());
}

#[tokio::test]
async fn test_generate_keys_with_mock() {
    let mock_context = MockIOContext::new().with_expected_io(vec![
        MockIO::eprint("🔑 Generating session keys via RPC..."),
        MockIO::eprint("✅ Generated session keys: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab5678901234567890ab5678901234"),
        MockIO::eprint("🔍 Decoding session keys..."),
        MockIO::eprint("  📝 Decoded aura key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
        MockIO::eprint("  📝 Decoded gran key: 0x5678901234567890ab5678901234567890ab5678901234567890ab5678901234"),
        MockIO::eprint("✅ Successfully decoded 2 session keys"),
        MockIO::write_file("session_keys.json"),
        MockIO::eprint("💾 Session keys saved to session_keys.json"),
        MockIO::eprint("🔑 Generated session keys:"),
        MockIO::print(""),
    ]);

    let keys_hex = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab5678901234567890ab5678901234";
    let mut mock_client = MockOnlineClient::from_url("ws://localhost:9944").await.unwrap();
    mock_client.expect_runtime_api()
        .once()
        .returning(|| {
            let mut runtime_api = MockRuntimeApi::new();
            runtime_api.expect_at_latest()
                .once()
                .returning(|| {
                    let mut block_ref = MockBlockRef::new();
                    block_ref.expect_call()
                        .once()
                        .with(eq(subxt::dynamic::runtime_api_call(
                            "SessionKeys",
                            "decode_session_keys",
                            vec![Value::from_bytes(hex::decode(keys_hex.strip_prefix("0x").unwrap()).unwrap())],
                        )))
                        .returning(|_| {
                            Ok(Value::unnamed_composite(vec![
                                Value::unnamed_composite(vec![
                                    Value::from_bytes(hex::decode("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap()),
                                    Value::from_bytes(b"aura".to_vec()),
                                ]),
                                Value::unnamed_composite(vec![
                                    Value::from_bytes(hex::decode("5678901234567890ab5678901234567890ab5678901234567890ab5678901234").unwrap()),
                                    Value::from_bytes(b"gran".to_vec()),
                                ]),
                            ]))
                        });
                    Ok(block_ref)
                });
            runtime_api
        });

    let mut mock_rpc = MockRpcClient::from_url("ws://localhost:9944").await.unwrap();
    mock_rpc.expect_call()
        .once()
        .with(eq("author_rotateKeys"), eq(()))
        .return_once(|_, _| Ok(keys_hex.to_string()));

    let config = AutomaticGenerateKeysConfig {
        node_url: "ws://localhost:9944".to_string(),
    };

    let result = generate_keys_via_rpc(&config, &mock_context).await;
    assert!(result.is_ok());
}

// Integration test (requires a running node, commented out)
#[cfg(test)]
#[tokio::test]
async fn test_integration_generate_keys() {
    // This requires a running substrate node with --rpc-methods=unsafe
    let config = AutomaticGenerateKeysConfig {
        node_url: "ws://localhost:9944".to_string(),
    };
    let mock_context = MockIOContext::new();

    // This would call the real RPC and runtime API
    let result = generate_keys_via_rpc(&config, &mock_context).await;
    // Assert based on real response
    assert!(result.is_ok());
}
