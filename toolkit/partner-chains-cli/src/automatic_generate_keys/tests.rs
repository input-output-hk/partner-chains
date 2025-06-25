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
		MockIO::eprint("✅ Generated session keys: 0x123abc"),
		MockIO::eprint("🔍 Decoding session keys to get key types..."),
		MockIO::eprint(r#"✅ Decode response: {"id":2,"jsonrpc":"2.0","result":"0x08deadbeef04aura"}"#),
		MockIO::eprint("⚠️  Runtime call returned encoded data - providing raw keys for now"),
		MockIO::eprint("✅ Successfully decoded 1 session keys"),
		MockIO::eprint("💾 Session keys saved to session_keys.json"),
		MockIO::eprint("🔑 Generated session keys:"),
		MockIO::print(r#"[
  {
    "key_type": "raw",
    "public_key": "0x123abc"
  }
]"#),
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

	// Mock the decode request
	let decode_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "state_call",
			"params": ["SessionKeys_decode_session_keys", "0x123abc"],
			"id": 2
		})))
		.with_body(
			r#"{"jsonrpc":"2.0","result":"0x08deadbeef04aura","id":2}"#,
		)
		.create();

	let config = AutomaticGenerateKeysConfig { node_url: server.url() };

	let result = generate_keys_via_rpc(&config, "", &mock_context);
	assert!(result.is_ok());

	rotate_mock.assert();
	decode_mock.assert();
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
		MockIO::eprint("🔍 Decoding session keys to get key types..."),
		MockIO::eprint(r#"✅ Decode response: {"id":2,"jsonrpc":"2.0","result":"0x00"}"#),
		MockIO::eprint("⚠️  Runtime call returned encoded data - providing raw keys for now"),
		MockIO::eprint("✅ Successfully decoded 1 session keys"),
		MockIO::eprint("💾 Session keys saved to session_keys.json"),
		MockIO::eprint("🔑 Generated session keys:"),
		MockIO::print(r#"[
  {
    "key_type": "raw",
    "public_key": "0x123abc"
  }
]"#),
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

	// Mock the decode request with empty result
	let decode_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "state_call",
			"params": ["SessionKeys_decode_session_keys", "0x123abc"],
			"id": 2
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x00","id":2}"#)
		.create();

	let config = AutomaticGenerateKeysConfig { node_url: server.url() };

	let result = generate_keys_via_rpc(&config, "", &mock_context);
	assert!(result.is_ok());

	rotate_mock.assert();
	decode_mock.assert();
}
