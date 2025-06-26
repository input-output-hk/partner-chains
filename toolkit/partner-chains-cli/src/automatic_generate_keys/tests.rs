use super::*;
use crate::tests::{MockIO, MockIOContext};
use mockito;
use serde_json::json;

#[test]
fn test_config_creation() {
	let mock_context = MockIOContext::new();
	let url = "http://example.com:9944".to_string();

	let config = AutomaticGenerateKeysConfig::load(&mock_context, url);
	assert_eq!(config.node_url, "http://example.com:9944");
}

#[test]
fn test_generate_keys_via_rpc() {
	let mut server = mockito::Server::new();
	let mock_context = MockIOContext::new().with_expected_io(vec![
		MockIO::eprint("🔑 Generating session keys via RPC..."),
		MockIO::eprint("✅ Generated session keys: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab5678901234567890ab5678901234567890ab"),
		MockIO::eprint("🔍 Parsing session keys..."),
		MockIO::eprint("  📝 Parsed aura key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"),
		MockIO::eprint("  📝 Parsed gran key: 0x5678901234567890ab5678901234567890ab5678901234567890ab567890123"),
		MockIO::eprint("  📝 Remaining data: 0x4567890ab"),
		MockIO::eprint("✅ Successfully parsed 3 session keys"),
		MockIO::eprint("💾 Session keys saved to session_keys.json"),
		MockIO::eprint("🔑 Generated session keys:"),
		MockIO::print(
			r#"[
  {
    "key_type": "aura",
    "public_key": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
  },
  {
    "key_type": "gran",
    "public_key": "0x5678901234567890ab5678901234567890ab5678901234567890ab567890123"
  },
  {
    "key_type": "remaining",
    "public_key": "0x4567890ab"
  }
]"#,
		),
	]);

	// Mock the rotate keys request
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab5678901234567890ab5678901234567890ab4567890ab","id":1}"#)
		.create();

	let config = AutomaticGenerateKeysConfig { node_url: server.url() };

	let result = generate_keys_via_rpc(&config, "", &mock_context);
	assert!(result.is_ok());

	rotate_mock.assert();
}

#[test]
fn test_generate_keys_error_response() {
	let mut server = mockito::Server::new();
	let mock_context = MockIOContext::new()
		.with_expected_io(vec![MockIO::eprint("🔑 Generating session keys via RPC...")]);

	// Mock the rotate keys request with an error response
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(
			r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"RPC call is unsafe to be called externally"},"id":1}"#,
		)
		.create();

	let config = AutomaticGenerateKeysConfig { node_url: server.url() };

	let result = generate_keys_via_rpc(&config, "", &mock_context);
	assert!(result.is_err());
	let error_msg = result.unwrap_err().to_string();
	assert!(error_msg.contains("RPC call is unsafe to be called externally"));
	assert!(error_msg.contains("--rpc-methods=unsafe"));

	rotate_mock.assert();
}

#[test]
fn test_generate_keys_empty_response() {
	let mut server = mockito::Server::new();
	let mock_context = MockIOContext::new().with_expected_io(vec![
		MockIO::eprint("🔑 Generating session keys via RPC..."),
		MockIO::eprint("✅ Generated session keys: 0x123abc"),
		MockIO::eprint("🔍 Parsing session keys..."),
		MockIO::eprint("  ⚠️  Could not parse individual keys - providing full hex as raw"),
		MockIO::eprint("✅ Successfully parsed 1 session keys"),
		MockIO::eprint("💾 Session keys saved to session_keys.json"),
		MockIO::eprint("🔑 Generated session keys:"),
		MockIO::print(
			r#"[
  {
    "key_type": "raw",
    "public_key": "0x123abc"
  }
]"#,
		),
	]);

	// Mock the rotate keys request
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x123abc","id":1}"#)
		.create();

	let config = AutomaticGenerateKeysConfig { node_url: server.url() };

	let result = generate_keys_via_rpc(&config, "", &mock_context);
	assert!(result.is_ok());

	rotate_mock.assert();
}
