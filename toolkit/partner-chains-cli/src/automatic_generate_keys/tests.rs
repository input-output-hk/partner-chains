use super::*;
use crate::tests::{MockIO, MockIOContext, RESOURCES_CONFIG_FILE_PATH};
use mockito;
use serde_json::json;

const DATA_PATH: &str = "./test_data";

fn keystore_path() -> String {
	format!("{}/keystore", DATA_PATH)
}

fn mock_config_loading() -> MockIO {
	MockIO::prompt("node base path", Some("./data"), DATA_PATH)
}

fn mock_config_loaded_from_file() -> MockIO {
	MockIO::eprint(&format!(
		"🛠️ Loaded node base path from config ({}): {}",
		RESOURCES_CONFIG_FILE_PATH, DATA_PATH
	))
}

fn mock_intro_messages() -> MockIO {
	MockIO::Group(vec![
		MockIO::eprint(
			"This 🧙 wizard will generate session keys by calling author_rotateKeys on the node, decode them, and save them to the keystore and partner-chains-public-keys.json file:",
		),
		MockIO::enewline(),
	])
}

fn mock_keystore_path_message() -> MockIO {
	MockIO::Group(vec![
		MockIO::eprint(&format!("🔑 Keystore path: {}", keystore_path())),
		MockIO::enewline(),
	])
}

#[test]
fn test_successful_key_generation() {
	let mut server = mockito::Server::new();

	let mock_context = MockIOContext::new()
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, serde_json::json!({"substrate_node_base_path": DATA_PATH}))
		.with_expected_io(vec![
			mock_intro_messages(),
			mock_config_loaded_from_file(),
			mock_keystore_path_message(),
			MockIO::print("Raw session keys (hex): 0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab"),
			MockIO::print("Saved aura key to ./test_data/keystore/617572611234567890abcdef1234567890abcdef"),
			MockIO::print("Saved gran key to ./test_data/keystore/6772616e5678901234567890ab5678901234567890ab"),
			MockIO::print("🔑 Public keys saved to partner-chains-public-keys.json:\n{\n  \"aura\": \"0x1234567890abcdef1234567890abcdef\",\n  \"gran\": \"0x5678901234567890ab5678901234567890ab\"\n}"),
			MockIO::print("You may share these public keys with your chain governance authority."),
			MockIO::print("Decoded session keys: {\"aura\": \"0x1234567890abcdef1234567890abcdef\", \"gran\": \"0x5678901234567890ab5678901234567890ab\"}"),
			MockIO::print("🚀 All done!"),
		]);

	// Mock author_rotateKeys
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab","id":1}"#)
		.create();

	// Mock chain_getFinalizedHead
	let finalized_head_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "chain_getFinalizedHead",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0xabcdef1234567890","id":1}"#)
		.create();

	// Mock state_call for SessionKeys_decode_session_keys
	let decode_keys_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::JsonString(
			json!({
				"jsonrpc": "2.0",
				"method": "state_call",
				"params": ["SessionKeys_decode_session_keys", "0x881234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab", "0xabcdef1234567890"],
				"id": 1
			}).to_string()
		))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x081061757261401234567890abcdef1234567890abcdef106772616e485678901234567890ab5678901234567890ab","id":1}"#)
		.create();

	let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };

	let result = cmd.run(&mock_context);
	match &result {
		Ok(()) => println!("Test passed!"),
		Err(e) => {
			eprintln!("Test failed with error: {}", e);
			eprintln!("Error source: {:?}", e.source());
		},
	}
	assert!(result.is_ok());

	rotate_mock.assert();
	finalized_head_mock.assert();
	decode_keys_mock.assert();
}

#[test]
fn test_rpc_error_handling() {
	let mut server = mockito::Server::new();

	let mock_context = MockIOContext::new()
		.with_json_file(
			RESOURCES_CONFIG_FILE_PATH,
			serde_json::json!({"substrate_node_base_path": DATA_PATH}),
		)
		.with_expected_io(vec![
			mock_intro_messages(),
			mock_config_loaded_from_file(),
			mock_keystore_path_message(),
		]);

	// Mock author_rotateKeys with error response
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"RPC call is unsafe to be called externally"},"id":1}"#)
		.create();

	let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };

	let result = cmd.run(&mock_context);
	assert!(result.is_err());
	let error_msg = result.unwrap_err().to_string();
	assert!(error_msg.contains("Failed to call author_rotateKeys"));
	assert!(error_msg.contains("RPC call is unsafe to be called externally"));

	rotate_mock.assert();
}

#[test]
fn test_no_keys_decoded() {
	let mut server = mockito::Server::new();

	let mock_context = MockIOContext::new()
		.with_json_file(
			RESOURCES_CONFIG_FILE_PATH,
			serde_json::json!({"substrate_node_base_path": DATA_PATH}),
		)
		.with_expected_io(vec![
			mock_intro_messages(),
			mock_config_loaded_from_file(),
			mock_keystore_path_message(),
			MockIO::print("Raw session keys (hex): 0x1234"),
			MockIO::eprint("⚠️ No session keys decoded. Saving raw keys as fallback."),
			MockIO::eprint("Please verify the node's runtime configuration by fetching metadata:"),
			MockIO::eprint("curl -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"state_getMetadata\",\"id\":1}' http://localhost:9933 > metadata.json"),
			MockIO::eprint("Look for the Session pallet and SessionKeys type to determine key order (e.g., aura, gran, imon)."),
			MockIO::print("Saved raw session keys to ./test_data/keystore/raw1234"),
			MockIO::print("🔑 Public keys saved to partner-chains-public-keys.json:\n{\n  \"raw\": \"0x1234\"\n}"),
			MockIO::print("You may share these public keys with your chain governance authority."),
			MockIO::print("Decoded session keys: {\"raw\": \"0x1234\"}"),
			MockIO::print("🚀 All done!"),
		]);

	// Mock author_rotateKeys
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x1234","id":1}"#)
		.create();

	// Mock chain_getFinalizedHead
	let finalized_head_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "chain_getFinalizedHead",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0xabcdef1234567890","id":1}"#)
		.create();

	// Mock state_call returning empty result
	let decode_keys_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::JsonString(
			json!({
				"jsonrpc": "2.0",
				"method": "state_call",
				"params": ["SessionKeys_decode_session_keys", "0x081234", "0xabcdef1234567890"],
				"id": 1
			})
			.to_string(),
		))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x00","id":1}"#)
		.create();

	let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };

	let result = cmd.run(&mock_context);
	assert!(result.is_ok());

	rotate_mock.assert();
	finalized_head_mock.assert();
	decode_keys_mock.assert();
}

#[test]
fn test_file_overwrite_declined() {
	let mut server = mockito::Server::new();

	let mock_context = MockIOContext::new()
		.with_file(KEYS_FILE_PATH, "existing content")
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, serde_json::json!({"substrate_node_base_path": DATA_PATH}))
		.with_expected_io(vec![
			mock_intro_messages(),
			mock_config_loaded_from_file(),
			mock_keystore_path_message(),
			MockIO::print("Raw session keys (hex): 0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab"),
			MockIO::print("Saved aura key to ./test_data/keystore/617572611234567890abcdef1234567890abcdef"),
			MockIO::prompt_yes_no("A keys file already exists at partner-chains-public-keys.json - overwrite it?", false, false),
			MockIO::print("Refusing to overwrite keys file - skipping JSON save"),
			MockIO::print("Decoded session keys: {\"aura\": \"0x1234567890abcdef1234567890abcdef\"}"),
			MockIO::print("🚀 All done!"),
		]);

	// Mock the RPC calls (similar to previous test but with simpler response)
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab","id":1}"#)
		.create();

	let finalized_head_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "chain_getFinalizedHead",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0xabcdef1234567890","id":1}"#)
		.create();

	let decode_keys_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::JsonString(
			json!({
				"jsonrpc": "2.0",
				"method": "state_call",
				"params": ["SessionKeys_decode_session_keys", "0x881234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab", "0xabcdef1234567890"],
				"id": 1
			}).to_string()
		))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x041061757261401234567890abcdef1234567890abcdef","id":1}"#)
		.create();

	let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };

	let result = cmd.run(&mock_context);
	assert!(result.is_ok());

	rotate_mock.assert();
	finalized_head_mock.assert();
	decode_keys_mock.assert();
}

#[test]
fn test_prompts_for_config_when_missing() {
	let mut server = mockito::Server::new();

	let mock_context = MockIOContext::new()
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, serde_json::json!({}))
		.with_expected_io(vec![
			mock_intro_messages(),
			mock_config_loading(),
			mock_keystore_path_message(),
			MockIO::print("Raw session keys (hex): 0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab"),
			MockIO::print("Saved aura key to ./test_data/keystore/617572611234567890abcdef1234567890abcdef"),
			MockIO::print("🔑 Public keys saved to partner-chains-public-keys.json:\n{\n  \"aura\": \"0x1234567890abcdef1234567890abcdef\"\n}"),
			MockIO::print("You may share these public keys with your chain governance authority."),
			MockIO::print("Decoded session keys: {\"aura\": \"0x1234567890abcdef1234567890abcdef\"}"),
			MockIO::print("🚀 All done!"),
		]);

	// Mock the RPC calls
	let rotate_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "author_rotateKeys",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab","id":1}"#)
		.create();

	let finalized_head_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::Json(json!({
			"jsonrpc": "2.0",
			"method": "chain_getFinalizedHead",
			"params": [],
			"id": 1
		})))
		.with_body(r#"{"jsonrpc":"2.0","result":"0xabcdef1234567890","id":1}"#)
		.create();

	let decode_keys_mock = server
		.mock("POST", "/")
		.match_body(mockito::Matcher::JsonString(
			json!({
				"jsonrpc": "2.0",
				"method": "state_call",
				"params": ["SessionKeys_decode_session_keys", "0x881234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab", "0xabcdef1234567890"],
				"id": 1
			}).to_string()
		))
		.with_body(r#"{"jsonrpc":"2.0","result":"0x041061757261401234567890abcdef1234567890abcdef","id":1}"#)
		.create();

	let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };

	let result = cmd.run(&mock_context);
	assert!(result.is_ok());

	rotate_mock.assert();
	finalized_head_mock.assert();
	decode_keys_mock.assert();
}
