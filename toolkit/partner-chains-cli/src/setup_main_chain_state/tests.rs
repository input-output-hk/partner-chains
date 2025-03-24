use crate::config::config_fields::CARDANO_PAYMENT_SIGNING_KEY_FILE;
use crate::config::CHAIN_CONFIG_FILE_PATH;
use crate::config::{config_fields, RESOURCES_CONFIG_FILE_PATH};
use crate::ogmios::config::tests::{
	default_ogmios_config_json, default_ogmios_service_config, prompt_ogmios_configuration_io,
};
use crate::prepare_configuration::tests::{prompt, prompt_with_default};
use crate::setup_main_chain_state::SetupMainChainStateCmd;
use crate::tests::{MockIO, MockIOContext, OffchainMock, OffchainMocks};
use crate::{verify_json, CmdRun};
use hex_literal::hex;
use partner_chains_cardano_offchain::multisig::MultiSigSmartContractResult;
use serde_json::json;
use sidechain_domain::{AuraPublicKey, DParameter, GrandpaPublicKey, SidechainPublicKey, UtxoId};
use sp_core::offchain::Timestamp;

#[test]
fn no_ariadne_parameters_on_main_chain_no_updates() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(ariadne_parameters_not_found_response()),
			print_ariadne_parameters_not_found_io(),
			prompt_permissioned_candidates_update_io(false),
			prompt_d_parameter_update_io(false),
			print_post_update_info_io(),
		]);
	let result = SetupMainChainStateCmd.run(&mock_context);

	result.expect("should succeed");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, no_updates_resources_json());
}

#[test]
fn no_ariadne_parameters_on_main_chain_do_updates() {
	let offchain_mock = OffchainMock::new()
		.with_upsert_d_param(
			UtxoId::default(),
			new_d_parameter(),
			payment_signing_key(),
			Ok(Some(MultiSigSmartContractResult::tx_submitted([1; 32]))),
		)
		.with_upsert_permissioned_candidates(
			genesis_utxo(),
			&initial_permissioned_candidates(),
			payment_signing_key(),
			Ok(Some(MultiSigSmartContractResult::tx_submitted([2; 32]))),
		);
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file("payment.skey", valid_payment_signing_key_content())
		.with_offchain_mocks(OffchainMocks::new_with_mock("http://localhost:1337", offchain_mock))
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(ariadne_parameters_not_found_response()),
			print_ariadne_parameters_not_found_io(),
			prompt_permissioned_candidates_update_io(true),
			upsert_permissioned_candidates_io(),
			prompt_d_parameter_update_io(true),
			insert_d_parameter_io(),
			print_post_update_info_io(),
		]);
	let result = SetupMainChainStateCmd.run(&mock_context);
	result.expect("should succeed");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, post_updates_resources_json());
}

#[test]
fn ariadne_parameters_are_on_main_chain_no_updates() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(ariadne_parameters_found_response()),
			print_main_chain_and_configuration_candidates_difference_io(),
			prompt_permissioned_candidates_update_io(false),
			print_d_param_from_main_chain_io(),
			prompt_d_parameter_update_io(false),
			print_post_update_info_io(),
		]);
	let result = SetupMainChainStateCmd.run(&mock_context);
	result.expect("should succeed");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, no_updates_resources_json());
}

#[test]
fn ariadne_parameters_are_on_main_chain_do_update() {
	let offchain_mock = OffchainMock::new()
		.with_upsert_d_param(
			UtxoId::default(),
			new_d_parameter(),
			payment_signing_key(),
			Ok(Some(MultiSigSmartContractResult::tx_submitted([1; 32]))),
		)
		.with_upsert_permissioned_candidates(
			genesis_utxo(),
			&initial_permissioned_candidates(),
			payment_signing_key(),
			Ok(Some(MultiSigSmartContractResult::tx_submitted([2; 32]))),
		);
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file("payment.skey", valid_payment_signing_key_content())
		.with_offchain_mocks(OffchainMocks::new_with_mock("http://localhost:1337", offchain_mock))
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(ariadne_parameters_found_response()),
			print_main_chain_and_configuration_candidates_difference_io(),
			prompt_permissioned_candidates_update_io(true),
			upsert_permissioned_candidates_io(),
			print_d_param_from_main_chain_io(),
			prompt_d_parameter_update_io(true),
			update_d_parameter_io(),
			print_post_update_info_io(),
		]);
	let result = SetupMainChainStateCmd.run(&mock_context);
	result.expect("should succeed");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, post_updates_resources_json());
}

#[test]
fn fails_if_update_permissioned_candidates_fail() {
	let offchain_mock = OffchainMock::new().with_upsert_permissioned_candidates(
		genesis_utxo(),
		&initial_permissioned_candidates(),
		payment_signing_key(),
		Err("something went wrong".into()),
	);
	let mock_context = MockIOContext::new()
		.with_offchain_mocks(OffchainMocks::new_with_mock("http://localhost:1337", offchain_mock))
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(ariadne_parameters_found_response()),
			print_main_chain_and_configuration_candidates_difference_io(),
			prompt_permissioned_candidates_update_io(true),
			upsert_permissioned_candidates_failed_io(),
		]);
	let result = SetupMainChainStateCmd.run(&mock_context);
	result.expect_err("should return error");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, post_updates_resources_json());
}

#[test]
fn candidates_on_main_chain_are_same_as_in_config_no_updates() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(ariadne_parameters_same_as_in_config_response()),
			print_main_chain_and_configuration_candidates_are_equal_io(),
			print_d_param_from_main_chain_io(),
			prompt_d_parameter_update_io(false),
			print_post_update_info_io(),
		]);
	let result = SetupMainChainStateCmd.run(&mock_context);
	result.expect("should succeed");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, no_updates_resources_json());
}

fn print_info_io() -> MockIO {
	MockIO::print("This wizard will set or update D-Parameter and Permissioned Candidates on the main chain. Setting either of these costs ADA!")
}

fn set_env_for_node_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::set_env_var(
			"DB_SYNC_POSTGRES_CONNECTION_STRING",
			"postgres://postgres:password123@localhost:5432/cexplorer",
		),
		MockIO::set_env_var("CARDANO_SECURITY_PARAMETER", "1234"),
		MockIO::set_env_var("CARDANO_ACTIVE_SLOTS_COEFF", "0.1"),
		MockIO::set_env_var("BLOCK_STABILITY_MARGIN", "0"),
		MockIO::set_env_var("MC__FIRST_EPOCH_TIMESTAMP_MILLIS", "1666742400000"),
		MockIO::set_env_var("MC__FIRST_EPOCH_NUMBER", "1"),
		MockIO::set_env_var("MC__EPOCH_DURATION_MILLIS", "86400000"),
		MockIO::set_env_var("MC__FIRST_SLOT_NUMBER", "4320"),
	])
}

fn get_ariadne_parameters_io(result: serde_json::Value) -> MockIO {
	let ariadne_parameters_command_output = serde_json::to_string(&result).unwrap();
	let timestamp_for_preview_epoch_605 = Timestamp::from_unix_millis(1_718_972_296_000u64);
	MockIO::Group(vec![
		MockIO::print("Will read the current D-Parameter and Permissioned Candidates from the main chain, using 'partner-chains-node ariadne-parameters' command."),
		prompt(config_fields::POSTGRES_CONNECTION_STRING, "postgres://postgres:password123@localhost:5432/cexplorer"),
		set_env_for_node_io(),
		MockIO::current_timestamp(timestamp_for_preview_epoch_605),
		MockIO::new_tmp_dir(),
		MockIO::run_command("<mock executable> ariadne-parameters --base-path /tmp/MockIOContext_tmp_dir --chain chain-spec.json --mc-epoch-number 607", &ariadne_parameters_command_output),
		MockIO::print(&ariadne_parameters_command_output),
	])
}

fn print_post_update_info_io() -> MockIO {
	MockIO::print("Done. Main chain state is set. Please remember that any changes can be observed immediately, but from the Partner Chain point of view they will be effective in two main chain epochs.")
}

fn prompt_d_parameter_update_io(choice: bool) -> MockIO {
	MockIO::prompt_yes_no(
		"Do you want to set/update the D-parameter on the main chain?",
		false,
		choice,
	)
}

fn prompt_permissioned_candidates_update_io(choice: bool) -> MockIO {
	MockIO::prompt_yes_no("Do you want to set/update the permissioned candidates on the main chain with values from configuration file?", false, choice)
}

fn upsert_permissioned_candidates_io() -> MockIO {
	MockIO::Group(vec![
		prompt_ogmios_configuration_io(
			&default_ogmios_service_config(),
			&default_ogmios_service_config(),
		),
		prompt(CARDANO_PAYMENT_SIGNING_KEY_FILE, "payment.skey"),
		MockIO::print(
			"Permissioned candidates updated. The change will be effective in two main chain epochs.",
		)])
}

fn upsert_permissioned_candidates_failed_io() -> MockIO {
	MockIO::Group(vec![
		prompt_ogmios_configuration_io(
			&default_ogmios_service_config(),
			&default_ogmios_service_config(),
		),
		prompt(CARDANO_PAYMENT_SIGNING_KEY_FILE, "payment.skey"),
	])
}

fn new_d_parameter() -> DParameter {
	DParameter::new(4, 7)
}

fn insert_d_parameter_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::prompt(
			"Enter P, the number of permissioned candidates seats, as a non-negative integer.",
			Some("0"),
			"4",
		),
		MockIO::prompt(
			"Enter R, the number of registered candidates seats, as a non-negative integer.",
			Some("0"),
			"7",
		),
		prompt(CARDANO_PAYMENT_SIGNING_KEY_FILE, "payment.skey"),
		MockIO::print(
			"D-parameter updated to (4, 7). The change will be effective in two main chain epochs.",
		),
	])
}

fn update_d_parameter_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::prompt(
			"Enter P, the number of permissioned candidates seats, as a non-negative integer.",
			Some("6"),
			"4",
		),
		MockIO::prompt(
			"Enter R, the number of registered candidates seats, as a non-negative integer.",
			Some("4"),
			"7",
		),
		prompt_with_default(CARDANO_PAYMENT_SIGNING_KEY_FILE, Some("payment.skey"), "payment.skey"),
		MockIO::print(
			"D-parameter updated to (4, 7). The change will be effective in two main chain epochs.",
		),
	])
}

fn print_main_chain_and_configuration_candidates_difference_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::print("Permissioned candidates in the pc-chain-config.json file does not match the most recent on-chain initial permissioned candidates."),
		MockIO::print("The most recent on-chain initial permissioned candidates are:"),
		MockIO::print("Partner Chains Key: 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1, AURA: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d, GRANDPA: 0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"),
		MockIO::print("Partner Chains Key: 0x0263c9cdabbef76829fe5b35f0bbf3051bd1c41b80f58b5d07c271d0dd04de2a4e, AURA: 0x9cedc9f7b926191f64d68ee77dd90c834f0e73c0f53855d77d3b0517041d5640, GRANDPA: 0xde21d8171821fc29a43a1ed90ee75623edc3794012010f165b6afc3483a569aa"),
		MockIO::print("The permissioned candidates in the configuration file are:"),
		MockIO::print("Partner Chains Key: 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1, AURA: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d, GRANDPA: 0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"),
		MockIO::print("Partner Chains Key: 0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27, AURA: 0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48, GRANDPA: 0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69"),
	])
}

fn print_main_chain_and_configuration_candidates_are_equal_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::print("Permissioned candidates in the pc-chain-config.json file match the most recent on-chain initial permissioned candidates."),
	])
}

fn print_d_param_from_main_chain_io() -> MockIO {
	MockIO::print("D-Parameter on the main chain is: (P=6, R=4)")
}

fn print_ariadne_parameters_not_found_io() -> MockIO {
	MockIO::print("Ariadne parameters not found.")
}

fn test_chain_config_content() -> serde_json::Value {
	json!({
		"chain_parameters": chain_parameters_json(),
		"cardano": {
			"security_parameter": 1234,
			"active_slots_coeff": 0.1,
			"first_epoch_timestamp_millis": 1_666_742_400_000_i64,
			"epoch_duration_millis": 86400000,
			"first_epoch_number": 1,
			"first_slot_number": 4320,
			"slot_duration_millis": 1000,
			"network": "testnet"
		},
		"cardano_addresses": {
			"committee_candidates_address": "addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc",
			"d_parameter_policy_id": "d0ebb61e2ba362255a7c4a253c6578884603b56fb0a68642657602d6",
			"permissioned_candidates_policy_id": "58b4ba68f641d58f7f1bba07182eca9386da1e88a34d47a14638c3fe",
			"native_token": {
				"asset": {
					"policy_id": "ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
					"asset_name": "5043546f6b656e44656d6f",
				},
				"illiquid_supply_address": "addr_test1wqn2pkvvmesmxtfa4tz7w8gh8vumr52lpkrhcs4dkg30uqq77h5z4"
			},
		},
		"initial_permissioned_candidates": [
			{
			  "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
			  "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
			  "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"
			},
			{
			  "aura_pub_key": "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
			  "grandpa_pub_key": "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69",
			  "sidechain_pub_key": "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27"
			}
		],
	})
}

fn post_updates_resources_json() -> serde_json::Value {
	json!({
		"cardano_payment_signing_key_file": "payment.skey",
		"db_sync_postgres_connection_string": "postgres://postgres:password123@localhost:5432/cexplorer",
		"ogmios": default_ogmios_config_json()
	})
}

fn no_updates_resources_json() -> serde_json::Value {
	json!({"db_sync_postgres_connection_string": "postgres://postgres:password123@localhost:5432/cexplorer"})
}

fn initial_permissioned_candidates() -> Vec<sidechain_domain::PermissionedCandidateData> {
	vec![
		sidechain_domain::PermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(
				hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec(),
			),
			aura_public_key: AuraPublicKey(
				hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").to_vec(),
			),
			grandpa_public_key: GrandpaPublicKey(
				hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee").to_vec(),
			),
		},
		sidechain_domain::PermissionedCandidateData {
			sidechain_public_key: SidechainPublicKey(
				hex!("0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27").to_vec(),
			),
			aura_public_key: AuraPublicKey(
				hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48").to_vec(),
			),
			grandpa_public_key: GrandpaPublicKey(
				hex!("d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69").to_vec(),
			),
		},
	]
}

fn genesis_utxo() -> UtxoId {
	UtxoId::new(hex!("0000000000000000000000000000000000000000000000000000000000000000"), 0)
}

fn chain_parameters_json() -> serde_json::Value {
	json!({
	  "genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0"
	})
}

fn test_resources_config_content() -> serde_json::Value {
	json!({})
}

fn ariadne_parameters_not_found_response() -> serde_json::Value {
	json!({
		"error": "ExpectedDataNotFound(DParameter)"
	})
}

fn ariadne_parameters_found_response() -> serde_json::Value {
	json!(
	{
			"dParameter": {
			  "numPermissionedCandidates": 6,
			  "numRegisteredCandidates": 4
			},
			"permissionedCandidates": [
			  {
				"sidechainPublicKey": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
				"auraPublicKey": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
				"grandpaPublicKey": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
				"isValid": true
			  },
			  {
				"sidechainPublicKey": "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27",
				"auraPublicKey": "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
				"grandpaPublicKey": "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69",
				"isValid": false
			  },
			  {
				"sidechainPublicKey": "0x0263c9cdabbef76829fe5b35f0bbf3051bd1c41b80f58b5d07c271d0dd04de2a4e",
				"auraPublicKey": "0x9cedc9f7b926191f64d68ee77dd90c834f0e73c0f53855d77d3b0517041d5640",
				"grandpaPublicKey": "0xde21d8171821fc29a43a1ed90ee75623edc3794012010f165b6afc3483a569aa",
				"isValid": true
			  }
			],
			"candidateRegistrations": {}
		}
			)
}

fn ariadne_parameters_same_as_in_config_response() -> serde_json::Value {
	json!(
	{
		"dParameter": {
		  "numPermissionedCandidates": 6,
		  "numRegisteredCandidates": 4
		},
		"permissionedCandidates": [
		  {
			"sidechainPublicKey": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
			"auraPublicKey": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
			"grandpaPublicKey": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
			"isValid": true
		  },
		  {
			"sidechainPublicKey": "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27",
			"auraPublicKey": "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
			"grandpaPublicKey": "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69",
			"isValid": true
		  }
		],
		"candidateRegistrations": {}
	}
		)
}

fn valid_payment_signing_key_content() -> serde_json::Value {
	json!(
		{
		"type": "PaymentSigningKeyShelley_ed25519",
		"description": "Payment Signing Key",
		"cborHex": "58200000000000000000000000000000000000000000000000000000000000000001"
	})
}

fn payment_signing_key() -> Vec<u8> {
	hex!("0000000000000000000000000000000000000000000000000000000000000001").to_vec()
}
