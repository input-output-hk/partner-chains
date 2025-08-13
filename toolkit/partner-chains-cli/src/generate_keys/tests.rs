use super::*;
use crate::CmdRun;
use crate::tests::runtime::MockRuntime;
use crate::tests::*;
use scenarios::key_file_content;
use scenarios::resources_file_content;

const DATA_PATH: &str = "/path/to/data";

const GRANDPA_PREFIX: &str = "65643235"; // "ed25" in hex
const CROSS_CHAIN_PREFIX: &str = "63726368"; // "crch" in hex
const AURA_PREFIX: &str = "73723235"; // "sr25" in hex

const AURA_KEY: &str = "070707070707070707070707070707070707070707070707070707070707070707";
const GRANDPA_KEY: &str = "0808080808080808080808080808080808080808080808080808080808080808";
const CROSS_CHAIN_KEY: &str = "1313131313131313131313131313131313131313131313131313131313131313";

fn default_config() -> GenerateKeysConfig {
	GenerateKeysConfig { substrate_node_base_path: DATA_PATH.into() }
}

fn network_key_file() -> String {
	format!("{DATA_PATH}/network/secret_ed25519")
}

fn keystore_path() -> String {
	format!("{DATA_PATH}/keystore")
}

pub mod scenarios {
	use super::*;

	pub fn show_intro() -> MockIO {
		MockIO::Group(vec![
			MockIO::eprint(
				"This 🧙 wizard will generate the following keys and save them to your node's keystore:",
			),
			MockIO::eprint("→ ecdsa Cross-chain key"),
			MockIO::eprint("→ ed25519 TestGrandpaLike key"),
			MockIO::eprint("→ sr25519 TestAuraLike key"),
			MockIO::eprint("It will also generate a network key for your node if needed."),
		])
	}

	pub fn prompt_all_config_fields() -> MockIO {
		MockIO::prompt("Enter the node base path", Some("./data"), DATA_PATH)
	}

	pub fn resources_file_content() -> serde_json::Value {
		serde_json::json!({"substrate_node_base_path": DATA_PATH})
	}

	pub fn generate_all_spo_keys(
		aura_key: &str,
		grandpa_key: &str,
		cross_chain_key: &str,
	) -> MockIO {
		MockIO::Group(vec![
			MockIO::list_dir(&keystore_path(), None),
			MockIO::eprint("⚙️ Generating Cross-chain (ecdsa) key"),
			MockIO::run_command_json(
				&"<mock executable> key generate --scheme ecdsa --output-type json".to_string(),
				&serde_json::json!({"publicKey": cross_chain_key, "secretPhrase": "cross-chain secret phrase"}),
			),
			MockIO::eprint("💾 Inserting Cross-chain (ecdsa) key"),
			MockIO::run_command(
				&format!(
					"<mock executable> key insert --chain /tmp/MockIOContext_tmp_dir/chain-spec.json --keystore-path {} --scheme ecdsa --key-type crch --suri 'cross-chain secret phrase'",
					keystore_path()
				),
				"",
			),
			MockIO::eprint(&format!(
				"💾 Cross-chain key stored at {}/{CROSS_CHAIN_PREFIX}{cross_chain_key}",
				&keystore_path()
			)),
			MockIO::enewline(),
			MockIO::list_dir(&keystore_path(), None),
			MockIO::eprint("⚙️ Generating TestGrandpaLike (ed25519) key"),
			MockIO::run_command_json(
				&"<mock executable> key generate --scheme ed25519 --output-type json".to_string(),
				&serde_json::json!({"publicKey": grandpa_key, "secretPhrase": "grandpa secret phrase"}),
			),
			MockIO::eprint("💾 Inserting TestGrandpaLike (ed25519) key"),
			MockIO::run_command(
				&format!(
					"<mock executable> key insert --chain /tmp/MockIOContext_tmp_dir/chain-spec.json --keystore-path {} --scheme ed25519 --key-type ed25 --suri 'grandpa secret phrase'",
					keystore_path()
				),
				"",
			),
			MockIO::eprint(&format!(
				"💾 TestGrandpaLike key stored at {}/{GRANDPA_PREFIX}{grandpa_key}",
				&keystore_path()
			)),
			MockIO::enewline(),
			MockIO::list_dir(&keystore_path(), None),
			MockIO::eprint("⚙️ Generating TestAuraLike (sr25519) key"),
			MockIO::run_command_json(
				&"<mock executable> key generate --scheme sr25519 --output-type json".to_string(),
				&serde_json::json!({"publicKey": aura_key, "secretPhrase": "aura secret phrase"}),
			),
			MockIO::eprint("💾 Inserting TestAuraLike (sr25519) key"),
			MockIO::run_command(
				&format!(
					"<mock executable> key insert --chain /tmp/MockIOContext_tmp_dir/chain-spec.json --keystore-path {} --scheme sr25519 --key-type sr25 --suri 'aura secret phrase'",
					keystore_path()
				),
				"",
			),
			MockIO::eprint(&format!(
				"💾 TestAuraLike key stored at {}/{AURA_PREFIX}{aura_key}",
				&keystore_path()
			)),
			MockIO::enewline(),
		])
	}

	pub fn generate_network_key() -> MockIO {
		MockIO::Group(vec![
			MockIO::eprint("⚙️ Generating network key"),
			MockIO::run_command(&format!("mkdir -p {DATA_PATH}/network"), "irrelevant"),
			MockIO::run_command(
				&format!(
					"<mock executable> key generate-node-key --chain /tmp/MockIOContext_tmp_dir/chain-spec.json --file {}",
					network_key_file()
				),
				"irrelevant",
			),
		])
	}
	pub fn key_file_content(aura: &str, grandpa: &str, cross_chain: &str) -> serde_json::Value {
		serde_json::json!({
			"partner_chains_key": format!("0x{cross_chain}"),
			"keys": {
				"ed25": format!("0x{grandpa}"),
				"sr25": format!("0x{aura}")
			}
		})
	}

	pub fn write_key_file(aura: &str, grandpa: &str, cross_chain: &str) -> MockIO {
		MockIO::Group(vec![
			MockIO::eprint(
				"🔑 The following public keys were generated and saved to the partner-chains-public-keys.json file:",
			),
			MockIO::print(&format!(
				"{{
  \"partner_chains_key\": \"0x{cross_chain}\",
  \"keys\": {{
    \"ed25\": \"0x{grandpa}\",
    \"sr25\": \"0x{aura}\"
  }}
}}"
			)),
			MockIO::eprint("You may share them with your chain governance authority"),
			MockIO::eprint("if you wish to be included as a permissioned candidate."),
		])
	}
}

#[test]
fn happy_path() {
	let mock_context = MockIOContext::new()
		.with_json_file(
			RESOURCES_CONFIG_FILE_PATH,
			serde_json::json!({
				"substrate_node_base_path": DATA_PATH,
			}),
		)
		.with_expected_io(vec![
			scenarios::show_intro(),
			MockIO::enewline(),
			MockIO::new_tmp_dir(),
			MockIO::eprint(&format!(
				"🛠️ Loaded node base path from config ({RESOURCES_CONFIG_FILE_PATH}): {DATA_PATH}"
			)),
			MockIO::enewline(),
			scenarios::generate_all_spo_keys(AURA_KEY, GRANDPA_KEY, CROSS_CHAIN_KEY),
			scenarios::write_key_file(AURA_KEY, GRANDPA_KEY, CROSS_CHAIN_KEY),
			MockIO::enewline(),
			scenarios::generate_network_key(),
			MockIO::enewline(),
			MockIO::eprint("🚀 All done!"),
			MockIO::delete_file("/tmp/MockIOContext_tmp_dir/chain-spec.json"),
		]);

	let result =
		GenerateKeysCmd::<MockRuntime> { 
			_phantom: std::marker::PhantomData,
			node_url: None,
		}.run(&mock_context);

	result.expect("should succeed");
	verify_json!(
		mock_context,
		"partner-chains-public-keys.json",
		key_file_content(AURA_KEY, GRANDPA_KEY, CROSS_CHAIN_KEY)
	);
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, resources_file_content());
}

mod config_read {
	use super::*;
	use pretty_assertions::assert_eq;

	#[test]
	fn prompts_for_each_when_missing() {
		let context =
			MockIOContext::new().with_expected_io(vec![scenarios::prompt_all_config_fields()]);

		let result = GenerateKeysConfig::load(&context);

		assert_eq!(result.substrate_node_base_path, DATA_PATH);
	}

	#[test]
	fn reads_all_when_present() {
		let context = MockIOContext::new()
			.with_json_file(
				RESOURCES_CONFIG_FILE_PATH,
				serde_json::json!({
					"substrate_node_base_path": DATA_PATH,
				}),
			)
			.with_expected_io(vec![MockIO::eprint(&format!(
				"🛠️ Loaded node base path from config ({RESOURCES_CONFIG_FILE_PATH}): {DATA_PATH}"
			))]);

		let result = GenerateKeysConfig::load(&context);

		assert_eq!(result.substrate_node_base_path, DATA_PATH);
	}
}

mod generate_spo_keys {
	use super::*;

	#[test]
	fn preserves_existing_keys_when_user_declines_overwrite() {
		let keystore_files = vec![
			format!("{CROSS_CHAIN_PREFIX}{CROSS_CHAIN_KEY}"),
			format!("{AURA_PREFIX}{AURA_KEY}"),
			format!("{GRANDPA_PREFIX}{GRANDPA_KEY}"),
		];
		let mock_context = MockIOContext::new()
			.with_json_file(
				RESOURCES_CONFIG_FILE_PATH,
				serde_json::json!({
					"substrate_node_base_path": DATA_PATH,
				}),
			)
			.with_expected_io(vec![
				MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
				MockIO::prompt_yes_no(
					&format!(
						"A Cross-chain key already exists in store: {CROSS_CHAIN_KEY} - overwrite it?"
					),
					false,
					false,
				),
				MockIO::enewline(),
				MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
				MockIO::prompt_yes_no(
					&format!(
						"A TestGrandpaLike key already exists in store: {GRANDPA_KEY} - overwrite it?"
					),
					false,
					false,
				),
				MockIO::enewline(),
				MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
				MockIO::prompt_yes_no(
					&format!(
						"A TestAuraLike key already exists in store: {AURA_KEY} - overwrite it?"
					),
					false,
					false,
				),
				MockIO::enewline(),
				scenarios::write_key_file(AURA_KEY, GRANDPA_KEY, CROSS_CHAIN_KEY),
			]);

		let result = generate_spo_keys::<MockIOContext, MockRuntime>(
			&default_config(),
			"irrelevant.json",
			&mock_context,
		);

		result.expect("should succeed");
		verify_json!(
			mock_context,
			"partner-chains-public-keys.json",
			key_file_content(AURA_KEY, GRANDPA_KEY, CROSS_CHAIN_KEY)
		);
	}

	#[test]
	fn skips_the_step_if_user_declines_keys_file_overwrite() {
		let mock_context = MockIOContext::new()
			.with_file(KEYS_FILE_PATH, "irrelevant")
			.with_json_file(
				RESOURCES_CONFIG_FILE_PATH,
				serde_json::json!({
					"substrate_node_base_path": DATA_PATH,
				}),
			)
			.with_expected_io(vec![
				MockIO::prompt_yes_no(
					&format! {"keys file {KEYS_FILE_PATH} exists - overwrite it?"},
					false,
					false,
				),
				MockIO::eprint("Refusing to overwrite keys file - skipping"),
			]);

		let result = generate_spo_keys::<MockIOContext, MockRuntime>(
			&default_config(),
			"irrelevant.json",
			&mock_context,
		);

		result.expect("should succeed");
	}
}

mod generate_network_key {
	use super::*;

	#[test]
	fn generates_new_key_when_missing() {
		let context = MockIOContext::new().with_expected_io(vec![
			MockIO::eprint("⚙️ Generating network key"),
			MockIO::run_command(&format!("mkdir -p {DATA_PATH}/network"), "irrelevant"),
			MockIO::run_command(
				&format!(
					"<mock executable> key generate-node-key --chain path/to/chain-spec.json --file {}",
					network_key_file()
				),
				"irrelevant",
			),
		]);

		let result = generate_network_key(&default_config(), "path/to/chain-spec.json", &context);

		assert!(result.is_ok());
	}

	#[test]
	fn skips_generation_when_valid_key_present() {
		// valid value produced by `key generate-network-key`
		let key = "584d548cae2b3a960b1e6b5233fc5e8cbadfc1823f8df0c2e96f830d255dbdf4";

		let context =
			MockIOContext::new().with_file(&network_key_file(), key).with_expected_io(vec![
				MockIO::eprint(
					"🔑 A valid network key is already present in the keystore, skipping generation",
				),
			]);

		let result = generate_network_key(&default_config(), "irrelevant.json", &context);

		assert!(result.is_ok());
	}

	#[test]
	fn regenerates_invalid_key() {
		// too long
		let key = "584d548cae2b3a960b1e6b5233fc5e8cbadfc1823f8df0c2e96f830d255dbdf42545223";

		let context =
			MockIOContext::new().with_file(&network_key_file(), key).with_expected_io(vec![
				MockIO::eprint(
					"⚠️ Network key in keystore is invalid (Invalid hex), wizard will regenerate it",
				),
				MockIO::eprint("⚙️ Regenerating the network key"),
				MockIO::delete_file(&network_key_file()),
				MockIO::run_command(&format!("mkdir -p {DATA_PATH}/network"), "irrelevant"),
				MockIO::run_command(
					&format!(
						"<mock executable> key generate-node-key --chain {}/chain_spec.json --file {}",
						DATA_PATH, network_key_file()
					),
					"irrelevant",
				),
			]);

		let result = generate_network_key(&default_config(), &format!("{}/chain_spec.json", DATA_PATH), &context);

		assert!(result.is_ok());
	}
}

#[test]
fn key_type_hex_works() {
	assert_eq!(GRANDPA.key_type_hex(), "6772616e")
}

// Tests for automatic key generation functionality
mod automatic_key_generation {
	use super::*;
	use anyhow::Result;
	use parity_scale_codec::Encode;

	#[tokio::test]
	async fn test_parse_decoded_keys_response_modern_format() -> Result<()> {
		// Test parsing modern Polkadot SDK format: Option<Vec<(Vec<u8>, u32)>>
		let key_data = vec![
			(b"aura_public_key".to_vec(), 0x61757261u32), // 'aura' as u32
			(b"grandpa_public_key".to_vec(), 0x6772616eu32), // 'gran' as u32
		];
		let encoded = Some(key_data).encode();
		
		let result = parse_decoded_keys_response(&encoded)?;
		
		assert_eq!(result.len(), 2);
		assert_eq!(result[0].0, 0x61757261u32.to_le_bytes().to_vec());
		assert_eq!(result[0].1, b"aura_public_key".to_vec());
		assert_eq!(result[1].0, 0x6772616eu32.to_le_bytes().to_vec());
		assert_eq!(result[1].1, b"grandpa_public_key".to_vec());
		
		Ok(())
	}

	#[tokio::test]
	async fn test_parse_decoded_keys_response_legacy_format() -> Result<()> {
		// Test parsing legacy format: Vec<(Vec<u8>, Vec<u8>)>
		let key_data = vec![
			(b"aura".to_vec(), b"aura_public_key".to_vec()),
			(b"gran".to_vec(), b"grandpa_public_key".to_vec()),
		];
		let encoded = key_data.encode();
		
		let result = parse_decoded_keys_response(&encoded)?;
		
		assert_eq!(result.len(), 2);
		assert_eq!(result[0].0, b"aura".to_vec());
		assert_eq!(result[0].1, b"aura_public_key".to_vec());
		assert_eq!(result[1].0, b"gran".to_vec());
		assert_eq!(result[1].1, b"grandpa_public_key".to_vec());
		
		Ok(())
	}

	#[tokio::test]
	async fn test_parse_decoded_keys_response_empty() -> Result<()> {
		// Test parsing empty response: Option<Vec<(Vec<u8>, u32)>> = None
		let encoded = Option::<Vec<(Vec<u8>, u32)>>::None.encode();
		
		let result = parse_decoded_keys_response(&encoded)?;
		
		assert_eq!(result.len(), 0);
		
		Ok(())
	}

	#[tokio::test]
	async fn test_parse_decoded_keys_response_invalid() {
		// Test parsing invalid data
		let invalid_data = b"invalid_data";
		
		let result = parse_decoded_keys_response(invalid_data);
		
		assert!(result.is_err());
		assert!(result.unwrap_err().to_string().contains("Failed to SCALE decode keys"));
	}

	// The core SCALE decoding functionality is thoroughly tested above.
	// The MockIO system in this codebase doesn't provide comprehensive file operation mocking,
	// so more detailed integration tests would require a separate test framework or actual
	// HTTP server setup for testing the full RPC workflow.
}
