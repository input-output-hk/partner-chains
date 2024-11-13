use super::*;
use crate::tests::*;
use crate::CmdRun;

const DATA_PATH: &str = "/path/to/data";
const EXECUTABLE_PATH: &str = "./partner-chains-node";
const RESOURCES_CONFIG_PATH: &str = "partner-chains-cli-resources-config.json";
const KEYS_FILE: &str = "partner-chains-public-keys.json";
const CHAIN_NAME: &str = "partner_chains_template";

const GRANDPA_PREFIX: &str = "6772616e"; // "gran" in hex
const CROSS_CHAIN_PREFIX: &str = "63726368"; // "crch" in hex
const AURA_PREFIX: &str = "61757261"; // "aura" in hex

fn default_config() -> GenerateKeysConfig {
	GenerateKeysConfig {
		chain_name: CHAIN_NAME.into(),
		substrate_node_base_path: DATA_PATH.into(),
		node_executable: EXECUTABLE_PATH.into(),
	}
}

fn network_key_file() -> String {
	format!("{DATA_PATH}/chains/{CHAIN_NAME}/network/secret_ed25519")
}

fn keystore_path() -> String {
	format!("{DATA_PATH}/chains/{CHAIN_NAME}/keystore")
}

pub mod scenarios {
	use super::*;

	pub fn show_intro() -> MockIO {
		MockIO::Group(vec![
			MockIO::eprint("This 🧙 wizard will generate the following keys and save them to your node's keystore:"),
			MockIO::eprint("→  an ECDSA Cross-chain key"),
			MockIO::eprint("→  an ED25519 Grandpa key"),
			MockIO::eprint("→  an SR25519 Aura key"),
			MockIO::eprint("It will also generate a network key for your node if needed.")
		])
	}

	pub fn prompt_all_config_fields() -> MockIO {
		MockIO::Group(vec![
			MockIO::file_write_json(
				RESOURCES_CONFIG_PATH,
				serde_json::json!({
					"substrate_node_executable_path": EXECUTABLE_PATH
				}),
			),
			MockIO::file_read(RESOURCES_CONFIG_PATH),
			MockIO::prompt("node base path", Some("./data"), DATA_PATH),
			MockIO::file_read(RESOURCES_CONFIG_PATH),
			MockIO::file_write_json(
				RESOURCES_CONFIG_PATH,
				serde_json::json!({
					"substrate_node_base_path": DATA_PATH,
					"substrate_node_executable_path": EXECUTABLE_PATH
				}),
			),
		])
	}

	pub fn generate_all_spo_keys(
		aura_key: &str,
		grandpa_key: &str,
		cross_chain_key: &str,
	) -> MockIO {
		MockIO::Group(vec![
			MockIO::list_dir(&keystore_path(), None),
			MockIO::eprint("⚙️ Generating Cross-chain (ecdsa) key"),
			MockIO::run_command_json(&format!("{EXECUTABLE_PATH} key generate --scheme ecdsa --output-type json"),
				&serde_json::json!({"publicKey": cross_chain_key, "secretPhrase": "cross-chain secret phrase"})),
			MockIO::eprint("💾 Inserting Cross-chain (ecdsa) key"),
			MockIO::run_command(&format!("{EXECUTABLE_PATH} key insert --base-path {DATA_PATH} --scheme ecdsa --key-type crch --suri 'cross-chain secret phrase'"), ""),
			MockIO::eprint(&format!("💾 Cross-chain key stored at {}/{CROSS_CHAIN_PREFIX}{cross_chain_key}", &keystore_path())),
			MockIO::enewline(),

			MockIO::list_dir(&keystore_path(), None),
			MockIO::eprint("⚙️ Generating Grandpa (ed25519) key"),
			MockIO::run_command_json(&format!("{EXECUTABLE_PATH} key generate --scheme ed25519 --output-type json"),
				&serde_json::json!({"publicKey": grandpa_key, "secretPhrase": "grandpa secret phrase"})),
			MockIO::eprint("💾 Inserting Grandpa (ed25519) key"),
			MockIO::run_command(&format!("{EXECUTABLE_PATH} key insert --base-path {DATA_PATH} --scheme ed25519 --key-type gran --suri 'grandpa secret phrase'"), ""),
			MockIO::eprint(&format!("💾 Grandpa key stored at {}/{GRANDPA_PREFIX}{grandpa_key}", &keystore_path())),
			MockIO::enewline(),

			MockIO::list_dir(&keystore_path(), None),
			MockIO::eprint("⚙️ Generating Aura (sr25519) key"),
			MockIO::run_command_json(&format!("{EXECUTABLE_PATH} key generate --scheme sr25519 --output-type json"),
				&serde_json::json!({"publicKey": aura_key, "secretPhrase": "aura secret phrase"})),
			MockIO::eprint("💾 Inserting Aura (sr25519) key"),
			MockIO::run_command(&format!("{EXECUTABLE_PATH} key insert --base-path {DATA_PATH} --scheme sr25519 --key-type aura --suri 'aura secret phrase'"), ""),
			MockIO::eprint(&format!("💾 Aura key stored at {}/{AURA_PREFIX}{aura_key}", &keystore_path())),
			MockIO::enewline(),
		])
	}

	pub fn generate_network_key() -> MockIO {
		MockIO::Group(vec![
			MockIO::file_read(&network_key_file()),
			MockIO::eprint("⚙️ Generating network key"),
			MockIO::run_command(
				&format!("{EXECUTABLE_PATH} key generate-node-key --base-path {DATA_PATH}"),
				"irrelevant",
			),
		])
	}

	pub fn write_key_file(aura: &str, grandpa: &str, cross_chain: &str) -> MockIO {
		MockIO::Group(vec![
			MockIO::file_write_json(
				"partner-chains-public-keys.json",
				serde_json::json!({
					"aura_pub_key": aura,
					"grandpa_pub_key": grandpa,
					"sidechain_pub_key": cross_chain,
				}),
			),
			MockIO::eprint("🔑 The following public keys were generated and saved to the partner-chains-public-keys.json file:"),
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
			MockIO::set_env_var("CHAIN_ID", "0"),
			MockIO::set_env_var("THRESHOLD_NUMERATOR", "0"),
			MockIO::set_env_var("THRESHOLD_DENOMINATOR", "0"),
			MockIO::set_env_var(
				"GENESIS_COMMITTEE_UTXO",
				"0000000000000000000000000000000000000000000000000000000000000000#0",
			),
			MockIO::set_env_var(
				"GOVERNANCE_AUTHORITY",
				"00000000000000000000000000000000000000000000000000000000",
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
	let mock_context = MockIOContext::new()
		.with_file(EXECUTABLE_PATH, "<mock executable>")
		.with_expected_io(vec![
			scenarios::show_intro(),
			MockIO::enewline(),
			scenarios::set_dummy_env(),
			scenarios::prompt_all_config_fields(),
			MockIO::enewline(),
			scenarios::generate_all_spo_keys(
				"aura-pub-key",
				"grandpa-pub-key",
				"cross-chain-pub-key",
			),
			scenarios::write_key_file("aura-pub-key", "grandpa-pub-key", "cross-chain-pub-key"),
			MockIO::enewline(),
			scenarios::generate_network_key(),
			MockIO::enewline(),
			MockIO::eprint("🚀 All done!"),
		]);

	let result = GenerateKeysCmd {}.run(&mock_context);

	result.expect("should succeed");
}

mod config_read {
	use super::*;
	use config::config_fields::NODE_EXECUTABLE_DEFAULT;
	use pretty_assertions::assert_eq;

	#[test]
	fn prompts_for_each_when_missing() {
		let context = MockIOContext::new()
			.with_file(NODE_EXECUTABLE_DEFAULT, "<mock executable>")
			.with_expected_io(vec![scenarios::prompt_all_config_fields()]);

		let result = GenerateKeysConfig::load(&context);

		assert_eq!(result.chain_name, CHAIN_NAME);
		assert_eq!(result.node_executable, EXECUTABLE_PATH);
		assert_eq!(result.substrate_node_base_path, DATA_PATH);
	}

	#[test]
	fn reads_all_when_present() {
		let context = MockIOContext::new()
			.with_file(NODE_EXECUTABLE_DEFAULT, "<mock executable>")
			.with_json_file(
				RESOURCES_CONFIG_PATH,
				serde_json::json!({
					"substrate_node_base_path": DATA_PATH,
					"substrate_node_executable_path": EXECUTABLE_PATH
				}),
			)
			.with_expected_io(vec![
				MockIO::file_read(RESOURCES_CONFIG_PATH),
				MockIO::file_read(RESOURCES_CONFIG_PATH),
				MockIO::eprint(&format!(
					"🛠️ Loaded node base path from config ({RESOURCES_CONFIG_PATH}): {DATA_PATH}"
				)),
			]);

		let result = GenerateKeysConfig::load(&context);

		assert_eq!(result.chain_name, CHAIN_NAME);
		assert_eq!(result.node_executable, EXECUTABLE_PATH);
		assert_eq!(result.substrate_node_base_path, DATA_PATH);
	}

	#[test]
	fn verify_executable_returns_error_when_node_executable_missing() {
		let context = MockIOContext::new();

		let result = verify_executable(&default_config(), &context);

		assert!(result.is_err());
		assert_eq!(
			result.unwrap_err().to_string(),
			"Partner Chains Node executable file (./partner-chains-node) is missing"
		)
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
				RESOURCES_CONFIG_PATH,
				serde_json::json!({
					"substrate_node_base_path": DATA_PATH,
					"substrate_node_executable_path": EXECUTABLE_PATH
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
	}

	#[test]
	fn skips_the_step_if_user_declines_keys_file_overwrite() {
		let mock_context = MockIOContext::new()
			.with_file(KEYS_FILE, "irrelevant")
			.with_json_file(
				RESOURCES_CONFIG_PATH,
				serde_json::json!({
					"substrate_node_base_path": DATA_PATH,
					"substrate_node_executable_path": EXECUTABLE_PATH
				}),
			)
			.with_expected_io(vec![
				MockIO::prompt_yes_no(
					&format! {"keys file {KEYS_FILE} exists - overwrite it?"},
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
		let context = MockIOContext::new()
			.with_file(EXECUTABLE_PATH, "<mock executable>")
			.with_expected_io(vec![
				MockIO::file_read(&network_key_file()),
				MockIO::eprint("⚙️ Generating network key"),
				MockIO::run_command(
					&format!("{EXECUTABLE_PATH} key generate-node-key --base-path {DATA_PATH}"),
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
				MockIO::file_read(&network_key_file()),
				MockIO::eprint("🔑 A valid network key is already present in the keystore, skipping generation")
			]);

		let result = generate_network_key(&default_config(), &context);

		assert!(result.is_ok());
	}

	#[test]
	fn regenerates_invalid_key() {
		// too long
		let key = "584d548cae2b3a960b1e6b5233fc5e8cbadfc1823f8df0c2e96f830d255dbdf42545223";

		let context = MockIOContext::new()
			.with_file(EXECUTABLE_PATH, "<mock executable>")
			.with_file(&network_key_file(), key)
			.with_expected_io(vec![
				MockIO::file_read(&network_key_file()),
				MockIO::eprint(
					"⚠️ Network key in keystore is invalid (Invalid hex), wizard will regenerate it",
				),
				MockIO::eprint("⚙️ Regenerating the network key"),
				MockIO::delete_file(&network_key_file()),
				MockIO::run_command(
					&format!("{EXECUTABLE_PATH} key generate-node-key --base-path {DATA_PATH}"),
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
