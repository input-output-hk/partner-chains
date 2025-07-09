use super::*;
use crate::CmdRun;
use crate::config::KEYS_FILE_PATH;
use crate::tests::*;
use crate::verify_json;
use mockito;
use parity_scale_codec::Encode;
use serde_json::json;

const DATA_PATH: &str = "/path/to/data";

const GRANDPA_PREFIX: &str = "6772616e"; // "gran" in hex
const AURA_PREFIX: &str = "61757261"; // "aura" in hex

fn keystore_path() -> String {
	format!("{DATA_PATH}/keystore")
}

pub mod scenarios {
	use super::*;

	pub fn show_intro() -> MockIO {
		MockIO::Group(vec![
			MockIO::eprint(
				"This 🧙 wizard will generate session keys by calling author_rotateKeys on the node, decode them, and save them to the keystore and partner-chains-public-keys.json file:",
			),
			MockIO::enewline(),
		])
	}

	pub fn prompt_all_config_fields() -> MockIO {
		MockIO::prompt("Enter the node base path", Some("./data"), DATA_PATH)
	}

	pub fn config_loaded_from_file() -> MockIO {
		MockIO::eprint(&format!(
			"🛠️ Loaded node base path from config ({}): {}",
			RESOURCES_CONFIG_FILE_PATH, DATA_PATH
		))
	}

	pub fn keystore_path_message() -> MockIO {
		MockIO::Group(vec![
			MockIO::eprint(&format!("🔑 Keystore path: {}", keystore_path())),
			MockIO::enewline(),
		])
	}

	pub fn resources_file_content() -> serde_json::Value {
		serde_json::json!({"substrate_node_base_path": DATA_PATH})
	}

	pub fn key_file_content(aura: &str, grandpa: &str) -> serde_json::Value {
		serde_json::json!({
			"aura": aura,
			"gran": grandpa,
		})
	}

	pub fn write_key_file(aura: &str, grandpa: &str) -> MockIO {
		MockIO::Group(vec![
			MockIO::print(&format!(
				"🔑 Public keys saved to {}:\n{{\n  \"aura\": \"{}\",\n  \"gran\": \"{}\"\n}}",
				KEYS_FILE_PATH, aura, grandpa
			)),
			MockIO::print("You may share these public keys with your chain governance authority."),
		])
	}

	pub fn generate_session_keys(aura: &str, grandpa: &str, raw_session_keys: &str) -> MockIO {
		let mut prints = vec![
			MockIO::print(&format!("Raw session keys (hex): {}", raw_session_keys)),
			MockIO::run_command(&format!("mkdir -p {}", keystore_path()), ""),
		];

		if !aura.is_empty() {
			prints.push(MockIO::print(&format!(
				"Saved aura key to {}/{}{}",
				keystore_path(),
				AURA_PREFIX,
				aura.strip_prefix("0x").unwrap_or(aura)
			)));
		}

		if !grandpa.is_empty() {
			prints.push(MockIO::print(&format!(
				"Saved gran key to {}/{}{}",
				keystore_path(),
				GRANDPA_PREFIX,
				grandpa.strip_prefix("0x").unwrap_or(grandpa)
			)));
		}

		MockIO::Group(prints)
	}

	pub fn write_raw_key_file(raw_key: &str) -> MockIO {
		MockIO::Group(vec![
			MockIO::print(&format!(
				"🔑 Public keys saved to {}:\n{{\n  \"raw\": \"{}\"\n}}",
				KEYS_FILE_PATH, raw_key
			)),
			MockIO::print("You may share these public keys with your chain governance authority."),
		])
	}

	pub fn completion_message() -> MockIO {
		MockIO::print("🚀 All done!")
	}

	pub fn setup_http_mocks(
		server: &mut mockito::Server,
		session_keys: &str,
		decode_response: &str,
	) {
		let _rotate_mock = server
			.mock("POST", "/")
			.match_body(mockito::Matcher::Json(json!({
				"jsonrpc": "2.0",
				"method": "author_rotateKeys",
				"params": [],
				"id": 1
			})))
			.with_body(&format!(r#"{{"jsonrpc":"2.0","result":"{}","id":1}}"#, session_keys))
			.create();

		let _finalized_head_mock = server
			.mock("POST", "/")
			.match_body(mockito::Matcher::Json(json!({
				"jsonrpc": "2.0",
				"method": "chain_getFinalizedHead",
				"params": [],
				"id": 1
			})))
			.with_body(r#"{"jsonrpc":"2.0","result":"0xabcdef1234567890","id":1}"#)
			.create();

		let _decode_keys_mock = server
			.mock("POST", "/")
			.match_body(mockito::Matcher::JsonString(
				json!({
					"jsonrpc": "2.0",
					"method": "state_call",
					"params": ["SessionKeys_decode_session_keys", &format!("0x{}", hex::encode(hex::decode(&session_keys[2..]).unwrap().encode())), "0xabcdef1234567890"],
					"id": 1
				}).to_string()
			))
			.with_body(&format!(r#"{{"jsonrpc":"2.0","result":"{}","id":1}}"#, decode_response))
			.create();
	}

	pub fn setup_error_http_mock(server: &mut mockito::Server, error_message: &str) {
		let _rotate_mock = server
			.mock("POST", "/")
			.match_body(mockito::Matcher::Json(json!({
				"jsonrpc": "2.0",
				"method": "author_rotateKeys",
				"params": [],
				"id": 1
			})))
			.with_body(&format!(
				r#"{{"jsonrpc":"2.0","error":{{"code":-32601,"message":"{}"}},"id":1}}"#,
				error_message
			))
			.create();
	}
}

#[test]
fn happy_path() {
	let mut server = mockito::Server::new();
	let aura_key = "0x1234567890abcdef1234567890abcdef";
	let grandpa_key = "0x5678901234567890ab5678901234567890ab";
	let session_keys = "0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab";

	// Setup HTTP mocks
	scenarios::setup_http_mocks(
		&mut server,
		session_keys,
		"0x081061757261401234567890abcdef1234567890abcdef106772616e485678901234567890ab5678901234567890ab",
	);

	let mock_context = MockIOContext::new()
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, scenarios::resources_file_content())
		.with_expected_io(vec![
			scenarios::show_intro(),
			scenarios::config_loaded_from_file(),
			scenarios::keystore_path_message(),
			scenarios::generate_session_keys(aura_key, grandpa_key, session_keys),
			scenarios::write_key_file(aura_key, grandpa_key),
			MockIO::print(&format!(
				"Decoded session keys: {{\"aura\": \"{}\", \"gran\": \"{}\"}}",
				aura_key, grandpa_key
			)),
			scenarios::completion_message(),
		]);

	let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };
	let result = cmd.run(&mock_context);

	result.expect("should succeed");
	verify_json!(mock_context, KEYS_FILE_PATH, scenarios::key_file_content(aura_key, grandpa_key));
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, scenarios::resources_file_content());
}

mod config_read {
	use super::*;

	#[test]
	fn prompts_for_each_when_missing() {
		let mut server = mockito::Server::new();
		let aura_key = "0x1234567890abcdef1234567890abcdef";
		let session_keys = "0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab";

		// Setup HTTP mocks
		scenarios::setup_http_mocks(
			&mut server,
			session_keys,
			"0x041061757261401234567890abcdef1234567890abcdef",
		);

		let mock_context = MockIOContext::new()
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, serde_json::json!({}))
			.with_expected_io(vec![
				scenarios::show_intro(),
				scenarios::prompt_all_config_fields(),
				scenarios::keystore_path_message(),
				scenarios::generate_session_keys(aura_key, "", session_keys), // Only aura key in this test
				MockIO::print(&format!(
					"🔑 Public keys saved to {}:\n{{\n  \"aura\": \"{}\"\n}}",
					KEYS_FILE_PATH, aura_key
				)),
				MockIO::print(
					"You may share these public keys with your chain governance authority.",
				),
				MockIO::print(&format!("Decoded session keys: {{\"aura\": \"{}\"}}", aura_key)),
				scenarios::completion_message(),
			]);

		let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };
		let result = cmd.run(&mock_context);

		result.expect("should succeed");
	}

	#[test]
	fn reads_all_when_present() {
		let mut server = mockito::Server::new();
		let aura_key = "0x1234567890abcdef1234567890abcdef";
		let session_keys = "0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab";

		// Setup HTTP mocks
		scenarios::setup_http_mocks(
			&mut server,
			session_keys,
			"0x041061757261401234567890abcdef1234567890abcdef",
		);

		let mock_context = MockIOContext::new()
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, scenarios::resources_file_content())
			.with_expected_io(vec![
				scenarios::show_intro(),
				scenarios::config_loaded_from_file(),
				scenarios::keystore_path_message(),
				scenarios::generate_session_keys(aura_key, "", session_keys), // Only aura key in this test
				MockIO::print(&format!(
					"🔑 Public keys saved to {}:\n{{\n  \"aura\": \"{}\"\n}}",
					KEYS_FILE_PATH, aura_key
				)),
				MockIO::print(
					"You may share these public keys with your chain governance authority.",
				),
				MockIO::print(&format!("Decoded session keys: {{\"aura\": \"{}\"}}", aura_key)),
				scenarios::completion_message(),
			]);

		let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };
		let result = cmd.run(&mock_context);

		result.expect("should succeed");
	}
}

mod automatic_generate_keys {
	use super::*;

	#[test]
	fn skips_the_step_if_user_declines_keys_file_overwrite() {
		let mut server = mockito::Server::new();
		let aura_key = "0x1234567890abcdef1234567890abcdef";
		let session_keys = "0x1234567890abcdef1234567890abcdef5678901234567890ab5678901234567890ab";

		// Setup HTTP mocks
		scenarios::setup_http_mocks(
			&mut server,
			session_keys,
			"0x041061757261401234567890abcdef1234567890abcdef",
		);

		let mock_context = MockIOContext::new()
			.with_file(KEYS_FILE_PATH, "existing content")
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, scenarios::resources_file_content())
			.with_expected_io(vec![
				scenarios::show_intro(),
				scenarios::config_loaded_from_file(),
				scenarios::keystore_path_message(),
				scenarios::generate_session_keys(aura_key, "", session_keys),
				MockIO::prompt_yes_no(
					&format!("A keys file already exists at {} - overwrite it?", KEYS_FILE_PATH),
					false,
					false,
				),
				MockIO::print("Refusing to overwrite keys file - skipping JSON save"),
				MockIO::print(&format!("Decoded session keys: {{\"aura\": \"{}\"}}", aura_key)),
				scenarios::completion_message(),
			]);

		let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };
		let result = cmd.run(&mock_context);

		result.expect("should succeed");
	}

	#[test]
	fn handles_rpc_error_gracefully() {
		let mut server = mockito::Server::new();

		// Setup error HTTP mock
		scenarios::setup_error_http_mock(&mut server, "RPC call is unsafe to be called externally");

		let mock_context = MockIOContext::new()
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, scenarios::resources_file_content())
			.with_expected_io(vec![
				scenarios::show_intro(),
				scenarios::config_loaded_from_file(),
				scenarios::keystore_path_message(),
			]);

		let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };
		let result = cmd.run(&mock_context);

		assert!(result.is_err());
		let error_msg = result.unwrap_err().to_string();
		assert!(error_msg.contains("Failed to call author_rotateKeys"));
		assert!(error_msg.contains("RPC call is unsafe to be called externally"));
	}

	#[test]
	fn handles_no_keys_decoded_fallback() {
		let mut server = mockito::Server::new();
		let raw_key = "0x1234";

		// Setup HTTP mocks for fallback scenario
		scenarios::setup_http_mocks(&mut server, raw_key, "0x00");

		let mock_context = MockIOContext::new()
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, scenarios::resources_file_content())
			.with_expected_io(vec![
				scenarios::show_intro(),
				scenarios::config_loaded_from_file(),
				scenarios::keystore_path_message(),
				MockIO::print(&format!("Raw session keys (hex): {}", raw_key)),
				MockIO::run_command(&format!("mkdir -p {}", keystore_path()), ""),
				MockIO::eprint("⚠️ No session keys decoded. Saving raw keys as fallback."),
				MockIO::eprint("Please verify the node's runtime configuration by fetching metadata:"),
				MockIO::eprint("curl -X POST -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"state_getMetadata\",\"id\":1}' http://localhost:9933 > metadata.json"),
				MockIO::eprint("Look for the Session pallet and SessionKeys type to determine key order (e.g., aura, gran, imon)."),
				MockIO::print(&format!("Saved raw session keys to {}/raw{}", keystore_path(), "1234")),
				scenarios::write_raw_key_file(raw_key),
				MockIO::print(&format!("Decoded session keys: {{\"raw\": \"{}\"}}", raw_key)),
				scenarios::completion_message(),
			]);

		let cmd = AutomaticGenerateKeysCmd { node_url: server.url() };
		let result = cmd.run(&mock_context);

		result.expect("should succeed");
		verify_json!(mock_context, KEYS_FILE_PATH, serde_json::json!({"raw": raw_key}));
	}
}
