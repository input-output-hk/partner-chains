use super::*;
use crate::CmdRun;
use crate::config::RESOURCES_CONFIG_FILE_PATH;
use crate::tests::*;
use scenarios::key_file_content;
use scenarios::resources_file_content;

const DATA_PATH: &str = "/path/to/data";

const GRANDPA_PREFIX: &str = "6772616e"; // "gran" in hex
const CROSS_CHAIN_PREFIX: &str = "63726368"; // "crch" in hex
const AURA_PREFIX: &str = "61757261"; // "aura" in hex

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
			MockIO::eprint("→  an ECDSA Cross-chain key"),
			MockIO::eprint("→  an ED25519 Grandpa key"),
			MockIO::eprint("→  an SR25519 Aura key"),
			MockIO::eprint("It will also generate a network key for your node if needed."),
		])
	}

	pub fn prompt_all_config_fields() -> MockIO {
		MockIO::prompt("node base path", Some("./data"), DATA_PATH)
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
					"<mock executable> key insert --keystore-path {} --scheme ecdsa --key-type crch --suri 'cross-chain secret phrase'",
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
			MockIO::eprint("⚙️ Generating Grandpa (ed25519) key"),
			MockIO::run_command_json(
				&"<mock executable> key generate --scheme ed25519 --output-type json".to_string(),
				&serde_json::json!({"publicKey": grandpa_key, "secretPhrase": "grandpa secret phrase"}),
			),
			MockIO::eprint("💾 Inserting Grandpa (ed25519) key"),
			MockIO::run_command(
				&format!(
					"<mock executable> key insert --keystore-path {} --scheme ed25519 --key-type gran --suri 'grandpa secret phrase'",
					keystore_path()
				),
				"",
			),
			MockIO::eprint(&format!(
				"💾 Grandpa key stored at {}/{GRANDPA_PREFIX}{grandpa_key}",
				&keystore_path()
			)),
			MockIO::enewline(),
			MockIO::list_dir(&keystore_path(), None),
			MockIO::eprint("⚙️ Generating Aura (sr25519) key"),
			MockIO::run_command_json(
				&"<mock executable> key generate --scheme sr25519 --output-type json".to_string(),
				&serde_json::json!({"publicKey": aura_key, "secretPhrase": "aura secret phrase"}),
			),
			MockIO::eprint("💾 Inserting Aura (sr25519) key"),
			MockIO::run_command(
				&format!(
					"<mock executable> key insert --keystore-path {} --scheme sr25519 --key-type aura --suri 'aura secret phrase'",
					keystore_path()
				),
				"",
			),
			MockIO::eprint(&format!(
				"💾 Aura key stored at {}/{AURA_PREFIX}{aura_key}",
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
				&format!("<mock executable> key generate-node-key --file {}", network_key_file()),
				"irrelevant",
			),
		])
	}
	pub fn key_file_content(aura: &str, grandpa: &str, cross_chain: &str) -> serde_json::Value {
		serde_json::json!({
			"sidechain_pub_key": cross_chain,
			"aura_pub_key": aura,
			"grandpa_pub_key": grandpa,
		})
	}
	pub fn write_key_file(aura: &str, grandpa: &str, cross_chain: &str) -> MockIO {
		MockIO::Group(vec![
			MockIO::eprint(
				"🔑 The following public keys were generated and saved to the partner-chains-public-keys.json file:",
			),
			MockIO::print(&format!(
				"{{
  \"sidechain_pub_key\": \"{cross_chain}\",
  \"aura_pub_key\": \"{aura}\",
  \"grandpa_pub_key\": \"{grandpa}\"
}}"
			)),
			MockIO::eprint("You may share them with your chain governance authority"),
			MockIO::eprint("if you wish to be included as a permissioned candidate."),
		])
	}

	pub fn set_dummy_env() -> MockIO {
		MockIO::Group(vec![
			MockIO::set_env_var(
				"GENESIS_UTXO",
				"0000000000000000000000000000000000000000000000000000000000000000#0",
			),
			MockIO::set_env_var("COMMITTEE_CANDIDATE_ADDRESS", "addr_10000"),
			MockIO::set_env_var(
				"D_PARAMETER_POLICY_ID",
				"00000000000000000000000000000000000000000000000000000000",
			),
			MockIO::set_env_var(
				"PERMISSIONED_CANDIDATES_POLICY_ID",
				"00000000000000000000000000000000000000000000000000000000",
			),
			MockIO::set_env_var(
				"NATIVE_TOKEN_POLICY_ID",
				"00000000000000000000000000000000000000000000000000000000",
			),
			MockIO::set_env_var(
				"NATIVE_TOKEN_ASSET_NAME",
				"00000000000000000000000000000000000000000000000000000000",
			),
			MockIO::set_env_var(
				"ILLIQUID_SUPPLY_VALIDATOR_ADDRESS",
				"00000000000000000000000000000000000000000000000000000000",
			),
		])
	}
}

#[test]
fn happy_path() {
	let mock_context = MockIOContext::new().with_expected_io(vec![
		scenarios::show_intro(),
		MockIO::enewline(),
		scenarios::set_dummy_env(),
		scenarios::prompt_all_config_fields(),
		MockIO::enewline(),
		scenarios::generate_all_spo_keys("aura-pub-key", "grandpa-pub-key", "cross-chain-pub-key"),
		scenarios::write_key_file("aura-pub-key", "grandpa-pub-key", "cross-chain-pub-key"),
		MockIO::enewline(),
		scenarios::generate_network_key(),
		MockIO::enewline(),
		MockIO::eprint("🚀 All done!"),
	]);

	let result = GenerateKeysCmd {}.run(&mock_context);

	result.expect("should succeed");
	verify_json!(
		mock_context,
		"partner-chains-public-keys.json",
		key_file_content("aura-pub-key", "grandpa-pub-key", "cross-chain-pub-key")
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
			format!("{CROSS_CHAIN_PREFIX}cross-chain-key"),
			format!("{AURA_PREFIX}aura-key"),
			format!("{GRANDPA_PREFIX}grandpa-key"),
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
					"A Cross-chain key already exists in store: cross-chain-key - overwrite it?",
					false,
					false,
				),
				MockIO::enewline(),
				MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
				MockIO::prompt_yes_no(
					"A Grandpa key already exists in store: grandpa-key - overwrite it?",
					false,
					false,
				),
				MockIO::enewline(),
				MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
				MockIO::prompt_yes_no(
					"A Aura key already exists in store: aura-key - overwrite it?",
					false,
					false,
				),
				MockIO::enewline(),
				scenarios::write_key_file("0xaura-key", "0xgrandpa-key", "0xcross-chain-key"),
			]);

		let result = generate_spo_keys(&default_config(), &mock_context);

		result.expect("should succeed");
		verify_json!(
			mock_context,
			"partner-chains-public-keys.json",
			key_file_content("0xaura-key", "0xgrandpa-key", "0xcross-chain-key")
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

		let result = generate_spo_keys(&default_config(), &mock_context);

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
				&format!("<mock executable> key generate-node-key --file {}", network_key_file()),
				"irrelevant",
			),
		]);

		let result = generate_network_key(&default_config(), &context);

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

		let result = generate_network_key(&default_config(), &context);

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
						"<mock executable> key generate-node-key --file {}",
						network_key_file()
					),
					"irrelevant",
				),
			]);

		let result = generate_network_key(&default_config(), &context);

		assert!(result.is_ok());
	}
}

#[test]
fn key_type_hex_works() {
	assert_eq!(GRANDPA.key_type_hex(), GRANDPA_PREFIX)
}
