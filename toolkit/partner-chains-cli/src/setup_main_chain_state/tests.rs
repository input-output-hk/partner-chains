use crate::config::config_fields::CARDANO_PAYMENT_SIGNING_KEY_FILE;
use crate::ogmios::config::tests::{
	default_ogmios_config_json, default_ogmios_service_config, prompt_ogmios_configuration_io,
};
use crate::prepare_configuration::tests::{prompt, prompt_with_default};
use crate::setup_main_chain_state::SetupMainChainStateCmd;
use crate::tests::runtime::MockRuntime;
use crate::tests::{
	CHAIN_CONFIG_FILE_PATH, MockIO, MockIOContext, OffchainMock, OffchainMocks,
	RESOURCES_CONFIG_FILE_PATH,
};
use crate::{CmdRun, CommonArguments, verify_json};
use hex_literal::hex;
use partner_chains_cardano_offchain::multisig::MultiSigSmartContractResult;
use serde_json::json;
use sidechain_domain::{
	CandidateKey, CandidateKeys, DParameter, PermissionedCandidateData, SidechainPublicKey, UtxoId,
};

#[test]
fn no_ariadne_parameters_on_main_chain_no_updates() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_offchain_mocks(OffchainMocks::new_with_mock(
			"http://localhost:1337",
			mock_with_ariadne_parameters_not_found(),
		))
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(),
			print_permissioned_candidates_are_not_set(),
			prompt_permissioned_candidates_update_io(false),
			prompt_d_parameter_update_io(false),
			print_post_update_info_io(),
		]);
	let result = setup_main_chain_state_cmd().run(&mock_context);

	result.expect("should succeed");
	verify_json!(
		mock_context,
		RESOURCES_CONFIG_FILE_PATH,
		resources_config_with_default_ogmios_config()
	);
}

#[test]
fn no_ariadne_parameters_on_main_chain_do_updates() {
	let offchain_mock = mock_with_ariadne_parameters_not_found()
		.with_upsert_d_param(
			genesis_utxo(),
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
			get_ariadne_parameters_io(),
			print_permissioned_candidates_are_not_set(),
			prompt_permissioned_candidates_update_io(true),
			upsert_permissioned_candidates_io(),
			prompt_d_parameter_update_io(true),
			insert_d_parameter_io(),
			print_post_update_info_io(),
		]);
	let result = setup_main_chain_state_cmd().run(&mock_context);
	result.expect("should succeed");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, post_updates_resources_json());
}

#[test]
fn ariadne_parameters_are_on_main_chain_no_updates() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_offchain_mocks(OffchainMocks::new_with_mock(
			"http://localhost:1337",
			mock_with_ariadne_parameters_found(),
		))
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(),
			print_main_chain_and_configuration_candidates_difference_io(),
			prompt_permissioned_candidates_update_io(false),
			print_d_param_from_main_chain_io(),
			prompt_d_parameter_update_io(false),
			print_post_update_info_io(),
		]);
	let result = setup_main_chain_state_cmd().run(&mock_context);
	result.expect("should succeed");
	verify_json!(
		mock_context,
		RESOURCES_CONFIG_FILE_PATH,
		resources_config_with_default_ogmios_config()
	);
}

#[test]
fn ariadne_parameters_are_on_main_chain_do_update() {
	let offchain_mock = mock_with_ariadne_parameters_found()
		.with_upsert_d_param(
			genesis_utxo(),
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
			get_ariadne_parameters_io(),
			print_main_chain_and_configuration_candidates_difference_io(),
			prompt_permissioned_candidates_update_io(true),
			upsert_permissioned_candidates_io(),
			print_d_param_from_main_chain_io(),
			prompt_d_parameter_update_io(true),
			update_d_parameter_io(),
			print_post_update_info_io(),
		]);
	let result = setup_main_chain_state_cmd().run(&mock_context);
	result.expect("should succeed");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, post_updates_resources_json());
}

#[test]
fn fails_if_update_permissioned_candidates_fail() {
	let offchain_mock = mock_with_ariadne_parameters_found().with_upsert_permissioned_candidates(
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
			get_ariadne_parameters_io(),
			print_main_chain_and_configuration_candidates_difference_io(),
			prompt_permissioned_candidates_update_io(true),
			upsert_permissioned_candidates_failed_io(),
		]);
	let result = setup_main_chain_state_cmd().run(&mock_context);
	result.expect_err("should return error");
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, post_updates_resources_json());
}

#[test]
fn candidates_on_main_chain_are_same_as_in_config_no_updates() {
	let mock_context = MockIOContext::new()
		.with_offchain_mocks(OffchainMocks::new_with_mock(
			"http://localhost:1337",
			mock_with_ariadne_parameters_same_as_in_config_response(),
		))
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_expected_io(vec![
			print_info_io(),
			get_ariadne_parameters_io(),
			print_main_chain_and_configuration_candidates_are_equal_io(),
			print_d_param_from_main_chain_io(),
			prompt_d_parameter_update_io(false),
			print_post_update_info_io(),
		]);
	let result = setup_main_chain_state_cmd().run(&mock_context);
	result.expect("should succeed");
	verify_json!(
		mock_context,
		RESOURCES_CONFIG_FILE_PATH,
		resources_config_with_default_ogmios_config()
	);
}

fn setup_main_chain_state_cmd() -> SetupMainChainStateCmd<MockRuntime> {
	SetupMainChainStateCmd {
		common_arguments: CommonArguments { retry_delay_seconds: 5, retry_count: 59 },
		_phantom: std::marker::PhantomData,
	}
}

fn print_info_io() -> MockIO {
	MockIO::print(
		"This wizard will set or update D-Parameter and Permissioned Candidates on the main chain. Setting either of these costs ADA!",
	)
}

fn get_ariadne_parameters_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::print(
			"Will read the current D-Parameter and Permissioned Candidates from the main chain using Ogmios client.",
		),
		prompt_ogmios_configuration_io(
			&default_ogmios_service_config(),
			&default_ogmios_service_config(),
		),
	])
}

fn print_post_update_info_io() -> MockIO {
	MockIO::print(
		"Done. Please remember that any changes to the Cardano state can be observed immediately, but from the Partner Chain point of view they will be effective in two main chain epochs.",
	)
}

fn prompt_d_parameter_update_io(choice: bool) -> MockIO {
	MockIO::prompt_yes_no(
		"Do you want to set/update the D-parameter on the main chain?",
		false,
		choice,
	)
}

fn prompt_permissioned_candidates_update_io(choice: bool) -> MockIO {
	MockIO::prompt_yes_no(
		"Do you want to set/update the permissioned candidates on the main chain with values from configuration file?",
		false,
		choice,
	)
}

fn upsert_permissioned_candidates_io() -> MockIO {
	MockIO::Group(vec![
		prompt(CARDANO_PAYMENT_SIGNING_KEY_FILE, "payment.skey"),
		MockIO::print(
			"Permissioned candidates updated. The change will be effective in two main chain epochs.",
		),
	])
}

fn upsert_permissioned_candidates_failed_io() -> MockIO {
	prompt(CARDANO_PAYMENT_SIGNING_KEY_FILE, "payment.skey")
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
		MockIO::print(
			"Permissioned candidates in the test-pc-chain-config.json file does not match the most recent on-chain initial permissioned candidates.",
		),
		MockIO::print("The most recent on-chain initial permissioned candidates are:"),
		MockIO::print(
			"Partner Chains Key: 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1, ed25: 0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee, sr25: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
		),
		MockIO::print(
			"Partner Chains Key: 0x0263c9cdabbef76829fe5b35f0bbf3051bd1c41b80f58b5d07c271d0dd04de2a4e, ed25: 0xde21d8171821fc29a43a1ed90ee75623edc3794012010f165b6afc3483a569aa, sr25: 0x9cedc9f7b926191f64d68ee77dd90c834f0e73c0f53855d77d3b0517041d5640",
		),
		MockIO::print("The permissioned candidates in the configuration file are:"),
		MockIO::print(
			"Partner Chains Key: 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1, ed25: 0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee, sr25: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
		),
		MockIO::print(
			"Partner Chains Key: 0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27, ed25: 0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69, sr25: 0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
		),
	])
}

fn print_main_chain_and_configuration_candidates_are_equal_io() -> MockIO {
	MockIO::Group(vec![MockIO::print(
		"Permissioned candidates in the test-pc-chain-config.json file match the most recent on-chain initial permissioned candidates.",
	)])
}

fn print_d_param_from_main_chain_io() -> MockIO {
	MockIO::print("D-Parameter on the main chain is: (P=6, R=4)")
}

fn print_permissioned_candidates_are_not_set() -> MockIO {
	MockIO::print("List of permissioned candidates is not set on Cardano yet.")
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
			"d_parameter_policy_id": "0xd0ebb61e2ba362255a7c4a253c6578884603b56fb0a68642657602d6",
			"permissioned_candidates_policy_id": "0x58b4ba68f641d58f7f1bba07182eca9386da1e88a34d47a14638c3fe",
			"bridge": {
				"asset": {
					"policy_id": "0xada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
					"asset_name": "0x5043546f6b656e44656d6f",
				},
				"illiquid_circulation_supply_validator_address": "addr_test1wqn2pkvvmesmxtfa4tz7w8gh8vumr52lpkrhcs4dkg30uqq77h5z4"
			},
		},
		"initial_permissioned_candidates": [
			{
				"keys":{
					"sr25": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
					"ed25": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee"
				},
				"partner_chains_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"
			},
			{
				"keys":{
					"sr25": "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
					"ed25": "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69"
				},
				"partner_chains_key": "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27"
			}
		],
	})
}

fn post_updates_resources_json() -> serde_json::Value {
	json!({
		"cardano_payment_signing_key_file": "payment.skey",
		"ogmios": default_ogmios_config_json()
	})
}

fn resources_config_with_default_ogmios_config() -> serde_json::Value {
	json!({"ogmios": default_ogmios_config_json()})
}

fn initial_permissioned_candidates() -> Vec<sidechain_domain::PermissionedCandidateData> {
	vec![candidate_data_1(), candidate_data_2()]
}

fn candidate_data_1() -> PermissionedCandidateData {
	PermissionedCandidateData {
		sidechain_public_key: SidechainPublicKey(
			hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec(),
		),
		keys: CandidateKeys(vec![
			CandidateKey {
				id: *b"sr25",
				bytes: hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d")
					.to_vec(),
			},
			CandidateKey {
				id: *b"ed25",
				bytes: hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee")
					.to_vec(),
			},
		]),
	}
}

fn candidate_data_2() -> PermissionedCandidateData {
	PermissionedCandidateData {
		sidechain_public_key: SidechainPublicKey(
			hex!("0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27").to_vec(),
		),
		keys: CandidateKeys(vec![
			CandidateKey {
				id: *b"sr25",
				bytes: hex!("8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48")
					.to_vec(),
			},
			CandidateKey {
				id: *b"ed25",
				bytes: hex!("d17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69")
					.to_vec(),
			},
		]),
	}
}

fn candidate_data_3() -> PermissionedCandidateData {
	PermissionedCandidateData {
		sidechain_public_key: SidechainPublicKey::from_hex_unsafe(
			"0263c9cdabbef76829fe5b35f0bbf3051bd1c41b80f58b5d07c271d0dd04de2a4e",
		),
		keys: CandidateKeys(vec![
			CandidateKey {
				id: *b"sr25",
				bytes: hex!("9cedc9f7b926191f64d68ee77dd90c834f0e73c0f53855d77d3b0517041d5640")
					.to_vec(),
			},
			CandidateKey {
				id: *b"ed25",
				bytes: hex!("de21d8171821fc29a43a1ed90ee75623edc3794012010f165b6afc3483a569aa")
					.to_vec(),
			},
		]),
	}
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

fn mock_with_ariadne_parameters_not_found() -> OffchainMock {
	OffchainMock::new()
		.with_get_d_param(genesis_utxo(), Ok(None))
		.with_get_permissioned_candidates(genesis_utxo(), Ok(None))
}

fn mock_with_ariadne_parameters_found() -> OffchainMock {
	OffchainMock::new()
		.with_get_d_param(
			genesis_utxo(),
			Ok(Some(DParameter { num_permissioned_candidates: 6, num_registered_candidates: 4 })),
		)
		.with_get_permissioned_candidates(
			genesis_utxo(),
			Ok(Some(vec![candidate_data_1(), candidate_data_3()])),
		)
}

fn mock_with_ariadne_parameters_same_as_in_config_response() -> OffchainMock {
	OffchainMock::new()
		.with_get_d_param(
			genesis_utxo(),
			Ok(Some(DParameter { num_permissioned_candidates: 6, num_registered_candidates: 4 })),
		)
		.with_get_permissioned_candidates(
			genesis_utxo(),
			Ok(Some(vec![candidate_data_1(), candidate_data_2()])),
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
