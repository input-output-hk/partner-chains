use super::*;
use crate::tests::{MockIO, MockIOContext};

#[test]
fn test_config_creation() {
	let mock_context = MockIOContext::new();
	let url = "http://example.com:9944".to_string();

	let config = AutomaticGenerateKeysConfig::load(&mock_context, url);
	assert_eq!(config.node_url, "http://example.com:9944");
}

#[test]
fn test_decode_session_keys_via_rpc() {
	let mock_context = MockIOContext::new().with_expected_io(vec![
        MockIO::run_command(
            r#"test-node rpc sessionKeys_decodeSessionKeys --params '["0x123abc"]' --url http://localhost:9944"#,
            r#"[["0x16c425233d22...", "gran"], ["0x2ef6a0d...", "imon"]]"#
        ),
        MockIO::eprint("✅ Decode response: [[\"0x16c425233d22...\", \"gran\"], [\"0x2ef6a0d...\", \"imon\"]]"),
        MockIO::eprint("✅ Successfully decoded 2 session keys"),
    ]);

	let result = decode_session_keys_via_rpc(
		"0x123abc",
		"test-node",
		"http://localhost:9944",
		&mock_context,
	)
	.unwrap();

	assert_eq!(result.len(), 2);

	// Check first key
	assert_eq!(result[0].key_type, "gran");
	assert_eq!(result[0].public_key, "0x16c425233d22...");

	// Check second key
	assert_eq!(result[1].key_type, "imon");
	assert_eq!(result[1].public_key, "0x2ef6a0d...");
}

#[test]
fn test_decode_empty_response() {
	let mock_context = MockIOContext::new().with_expected_io(vec![
        MockIO::run_command(
            r#"test-node rpc sessionKeys_decodeSessionKeys --params '["0x123abc"]' --url http://localhost:9944"#,
            "[]"
        ),
        MockIO::eprint("✅ Decode response: []"),
        MockIO::eprint("✅ Successfully decoded 0 session keys"),
    ]);

	let result = decode_session_keys_via_rpc(
		"0x123abc",
		"test-node",
		"http://localhost:9944",
		&mock_context,
	)
	.unwrap();

	assert_eq!(result.len(), 0);
}

#[test]
fn test_decode_invalid_json() {
	let mock_context = MockIOContext::new().with_expected_io(vec![
        MockIO::run_command(
            r#"test-node rpc sessionKeys_decodeSessionKeys --params '["0x123abc"]' --url http://localhost:9944"#,
            "invalid json"
        ),
        MockIO::eprint("✅ Decode response: invalid json"),
    ]);

	let result = decode_session_keys_via_rpc(
		"0x123abc",
		"test-node",
		"http://localhost:9944",
		&mock_context,
	);

	assert!(result.is_err());
	assert!(
		result
			.unwrap_err()
			.to_string()
			.contains("Failed to parse decode response as JSON")
	);
}
