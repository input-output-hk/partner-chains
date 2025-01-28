use crate::tests::{MockIO, MockIOContext};

use super::*;

const RESOURCES_CONFIG_PATH: &str = "partner-chains-cli-resources-config.json";
const DATA_PATH: &str = "/path/to/data";
const CHAIN_SPEC_FILE: &str = "chain-spec.json";
const DB_CONNECTION_STRING: &str =
	"postgresql://postgres-user:postgres-password@localhost:5432/cexplorer";
const SIDECHAIN_BLOCK_BENEFICIARY_STRING: &str =
	"01e552298e47454041ea31273b4b630c64c104e4514aa3643490b8aaca9cf8ed";
fn keystore_path() -> String {
	format!("{DATA_PATH}/chains/{DEFAULT_CHAIN_NAME}/keystore")
}
const GRANDPA_PREFIX: &str = "6772616e"; // "gran" in hex
const CROSS_CHAIN_PREFIX: &str = "63726368"; // "crch" in hex
const AURA_PREFIX: &str = "61757261"; // "aura" in hex

fn default_config() -> StartNodeConfig {
	StartNodeConfig { substrate_node_base_path: DATA_PATH.into() }
}

fn default_config_json() -> serde_json::Value {
	serde_json::json!({
		"substrate_node_base_path": DATA_PATH,
		"db_sync_postgres_connection_string": DB_CONNECTION_STRING
	})
}

const BOOTNODE: &str =
	"/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp";
const SECURITY_PARAMETER: u64 = 432;
const ACTIVE_SLOTS_COEFF: f64 = 0.05;
const FIRST_EPOCH_NUMBER: u64 = 5;
const FIRST_SLOT_NUMBER: u64 = 42000;
const EPOCH_DURATION_MILLIS: u64 = 43200;
const FIRST_EPOCH_TIMESTAMP_MILLIS: u64 = 1590000000000;

fn default_chain_config() -> serde_json::Value {
	serde_json::json!({
		"bootnodes": [
			"/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp"
		],
		"cardano": {
			"network": "testnet",
			"security_parameter": SECURITY_PARAMETER,
			"active_slots_coeff": ACTIVE_SLOTS_COEFF,
			"first_epoch_number": FIRST_EPOCH_NUMBER,
			"first_slot_number": FIRST_SLOT_NUMBER,
			"epoch_duration_millis": EPOCH_DURATION_MILLIS,
			"first_epoch_timestamp_millis": FIRST_EPOCH_TIMESTAMP_MILLIS
		},
		"chain_parameters": {
			"genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0",
		}
	})
}

fn default_chain_config_run_command() -> String {
	let node_ws_port = NODE_P2P_PORT.default.unwrap();
	format!(
		"CARDANO_SECURITY_PARAMETER='{SECURITY_PARAMETER}' \\
         CARDANO_ACTIVE_SLOTS_COEFF='{ACTIVE_SLOTS_COEFF}' \\
         DB_SYNC_POSTGRES_CONNECTION_STRING='{DB_CONNECTION_STRING}' \\
         MC__FIRST_EPOCH_TIMESTAMP_MILLIS='{FIRST_EPOCH_TIMESTAMP_MILLIS}' \\
         MC__EPOCH_DURATION_MILLIS='{EPOCH_DURATION_MILLIS}' \\
         MC__FIRST_EPOCH_NUMBER='{FIRST_EPOCH_NUMBER}' \\
         MC__FIRST_SLOT_NUMBER='{FIRST_SLOT_NUMBER}' \\
         BLOCK_STABILITY_MARGIN='0' \\
		 SIDECHAIN_BLOCK_BENEFICIARY='{SIDECHAIN_BLOCK_BENEFICIARY_STRING}' \\
 <mock executable> --validator --chain {CHAIN_SPEC_FILE} --base-path {DATA_PATH} --port {node_ws_port} --bootnodes {BOOTNODE}"
	)
}

#[rustfmt::skip]
fn value_check_prompt() -> MockIO {
    MockIO::Group(vec![
        MockIO::eprint("The following values will be used to run the node:"),
        MockIO::eprint(&format!("    base path  = {}", DATA_PATH)),
        MockIO::eprint(&format!("    chain spec = {}", CHAIN_SPEC_PATH)),
		MockIO::eprint(&format!("    bootnodes  = [{}]", BOOTNODE)),
        MockIO::eprint("    environment:"),
        MockIO::eprint(&format!("        BLOCK_STABILITY_MARGIN             = {}", 0)),
        MockIO::eprint(&format!("        CARDANO_SECURITY_PARAMETER         = {}", SECURITY_PARAMETER)),
        MockIO::eprint(&format!("        CARDANO_ACTIVE_SLOTS_COEFF         = {}", ACTIVE_SLOTS_COEFF)),
        MockIO::eprint(&format!("        FIRST_EPOCH_TIMESTAMP_MILLIS       = {}", FIRST_EPOCH_TIMESTAMP_MILLIS)),
        MockIO::eprint(&format!("        EPOCH_DURATION_MILLIS              = {}", EPOCH_DURATION_MILLIS)),
        MockIO::eprint(&format!("        FIRST_EPOCH_NUMBER                 = {}", FIRST_EPOCH_NUMBER)),
        MockIO::eprint(&format!("        FIRST_SLOT_NUMBER                  = {}", FIRST_SLOT_NUMBER)),
        MockIO::eprint(&format!("        DB_SYNC_POSTGRES_CONNECTION_STRING = {}", DB_CONNECTION_STRING)),
	    MockIO::eprint(&format!("        SIDECHAIN_BLOCK_BENEFICIARY        = {}", SIDECHAIN_BLOCK_BENEFICIARY_STRING)),
        MockIO::prompt_yes_no("Proceed?", true, true)

    ])
}

#[test]
fn happy_path() {
	let keystore_files = vec![
		format!("{CROSS_CHAIN_PREFIX}020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"),
		format!("{AURA_PREFIX}aura-key"),
		format!("{GRANDPA_PREFIX}grandpa-key"),
	];

	let context = MockIOContext::new()
		.with_json_file(RESOURCES_CONFIG_PATH, default_config_json())
        .with_json_file(CHAIN_CONFIG_FILE_PATH, default_chain_config())
		.with_file(CHAIN_SPEC_FILE, "irrelevant")
		.with_expected_io(vec![
			MockIO::eprint(&format!(
				"🛠️ Loaded node base path from config ({RESOURCES_CONFIG_PATH}): {DATA_PATH}"
			)),
			MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
			MockIO::eprint(&format!(
				"🛠️ Loaded DB-Sync Postgres connection string from config ({RESOURCES_CONFIG_PATH}): {DB_CONNECTION_STRING}"
			)),
			MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
			MockIO::file_write_json_contains(RESOURCES_CONFIG_PATH, &SIDECHAIN_BLOCK_BENEFICIARY.json_pointer(), SIDECHAIN_BLOCK_BENEFICIARY_STRING),
			value_check_prompt(),
			MockIO::file_write_json_contains(RESOURCES_CONFIG_PATH, &NODE_P2P_PORT.json_pointer(), NODE_P2P_PORT.default.unwrap()),
			MockIO::run_command(&default_chain_config_run_command(), "irrelevant")
		]);

	let result = StartNodeCmd { silent: false }.run(&context);

	result.expect("should succeed");
}

mod check_chain_spec {

	use super::*;

	#[test]
	fn passes_if_present() {
		let context = MockIOContext::new().with_file(CHAIN_SPEC_FILE, "irrelevant");
		let result = check_chain_spec(&context);

		assert!(result);
	}

	#[test]
	fn fails_if_not_present() {
		let context = MockIOContext::new().with_expected_io(vec![
			MockIO::eprint("Chain spec file chain-spec.json missing."),
			MockIO::eprint("Please run the create-chain-spec wizard first or you can get it from your chain governance."),
		]);

		let result = check_chain_spec(&context);

		assert!(!result);
	}
}

mod check_keystore {
	use crate::tests::MockIOContext;

	use super::*;

	#[test]
	fn passes_when_all_present() {
		let keystore_files = vec![
			format!("{CROSS_CHAIN_PREFIX}cross-chain-key"),
			format!("{AURA_PREFIX}aura-key"),
			format!("{GRANDPA_PREFIX}grandpa-key"),
		];
		#[rustfmt::skip]
		let context = MockIOContext::new().with_expected_io(vec![
			MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
		]);

		let result = check_keystore(&default_config(), &context);

		result.expect("should succeed");
	}

	#[test]
	fn fails_when_one_is_missing() {
		let keystore_files = vec![
			format!("{CROSS_CHAIN_PREFIX}cross-chain-key"),
			format!("{GRANDPA_PREFIX}grandpa-key"),
		];
		let context = MockIOContext::new().with_expected_io(vec![
			MockIO::list_dir(&keystore_path(), Some(keystore_files.clone())),
			MockIO::eprint(
				"⚠️ Aura key is missing from the keystore. Please run generate-keys wizard first.",
			),
		]);

		let result = check_keystore(&default_config(), &context);
		let result = result.expect("should succeed");
		assert!(!result);
	}
}

mod load_chain_config {
	use crate::{
		config::CHAIN_CONFIG_FILE_PATH,
		start_node::load_chain_config,
		tests::{MockIO, MockIOContext},
	};

	use super::default_chain_config;

	#[test]
	fn accepts_a_correct_config() {
		let context =
			MockIOContext::new().with_json_file(CHAIN_CONFIG_FILE_PATH, default_chain_config());

		let result = load_chain_config(&context);

		result.expect("should succeed").expect("should return a value");
	}

	#[test]
	fn aborts_when_missing() {
		let context =
			MockIOContext::new().with_expected_io(vec![
                MockIO::eprint(&format!(
                    "⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} does not exists. Run prepare-configuration wizard first."
                ))
            ]);

		let result = load_chain_config(&context);

		assert!(result.expect("should succeed").is_none());
	}

	#[test]
	fn rejects_incorrect_config() {
		let mut incorrect = default_chain_config();
		incorrect.as_object_mut().unwrap().remove("cardano");

		let context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_FILE_PATH, incorrect)
			.with_expected_io(vec![
                MockIO::eprint(&format!(
                    "⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} is invalid: missing field `cardano` at line 8 column 1. Run prepare-configuration wizard or fix errors manually."
                ))
            ]);

		let result = load_chain_config(&context);

		assert!(result.expect("should succeed").is_none(), "should not return a config value");
	}
}
