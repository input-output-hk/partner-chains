use crate::config::{CHAIN_CONFIG_FILE_PATH, RESOURCES_CONFIG_FILE_PATH};
use crate::deregister::DeregisterCmd;
use crate::pc_contracts_cli_resources::tests::establish_pc_contracts_cli_configuration_io;
use crate::pc_contracts_cli_resources::PcContractsCliResources;
use crate::tests::{MockIO, MockIOContext};
use crate::CmdRun;
use serde_json::json;

const MY_PAYMEMENT_SKEY: &str = "my_payment.skey";
const MY_COLD_VKEY: &str = "my_cold.vkey";

#[test]
fn happy_path() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file(MY_PAYMEMENT_SKEY, valid_payment_signing_key_content())
		.with_json_file(MY_COLD_VKEY, valid_cold_verification_key_content())
		.with_expected_io(vec![
			read_config_twice_io(),
			print_info_io(),
			read_keys_io(),
			establish_pc_contracts_cli_configuration_io(None, PcContractsCliResources::default()),
			run_smart_contract_io(r#"{"endpoint":"CommitteeCandidateDereg","transactionId":"9aebb6d1d7f92f773f7d3025dd2fca67804ad6aea4a84a7696cd5ad15a4ee432"}"#),
			MockIO::print(r#"Deregistration successful: {"endpoint":"CommitteeCandidateDereg","transactionId":"9aebb6d1d7f92f773f7d3025dd2fca67804ad6aea4a84a7696cd5ad15a4ee432"}"#),
		]);
	let result = DeregisterCmd.run(&mock_context);
	assert!(result.is_ok());
}

#[test]
fn errors_if_smart_contracts_dont_output_transaction_id() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file(MY_PAYMEMENT_SKEY, valid_payment_signing_key_content())
		.with_json_file(MY_COLD_VKEY, valid_cold_verification_key_content())
		.with_expected_io(vec![
			read_config_twice_io(),
			print_info_io(),
			read_keys_io(),
			establish_pc_contracts_cli_configuration_io(None, PcContractsCliResources::default()),
			run_smart_contract_io(r#"(NotFoundInputUtxo "Couldn't find registration UTxO")"#),
		]);
	let result = DeregisterCmd.run(&mock_context);
	assert_eq!(
		result.err().unwrap().to_string(),
		r#"Deregistration failed: (NotFoundInputUtxo "Couldn't find registration UTxO")"#
	);
}

#[test]
fn fails_when_chain_config_is_not_valid() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, invalid_chain_config_content())
		.with_expected_io(vec![read_config_twice_io()]);
	let result = DeregisterCmd.run(&mock_context);
	assert_eq!(
	    result.err().unwrap().to_string(),
		"Couldn't parse chain configuration file partner-chains-cli-chain-config.json. The chain configuration file that was used for registration is required in the working directory."
	);
}

#[test]
fn fails_when_payment_signing_key_is_not_valid() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_file(MY_PAYMEMENT_SKEY, "not a proper Cardano key json")
		.with_expected_io(vec![
            read_config_twice_io(),
			print_info_io(),
            MockIO::print("Payment signing key and verification key of cold key used for registration are required to deregister."),
            read_payment_signing_key()
		]);
	let result = DeregisterCmd.run(&mock_context);
	assert_eq!(
		result.err().unwrap().to_string(),
		"my_payment.skey is not a valid Cardano key file: expected ident at line 1 column 2"
	);
}

#[test]
fn fails_when_cold_key_is_not_valid() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file(MY_PAYMEMENT_SKEY, valid_payment_signing_key_content())
		.with_file(MY_COLD_VKEY, "not a proper Cardano key json")
		.with_expected_io(vec![read_config_twice_io(), print_info_io(), read_keys_io()]);
	let result = DeregisterCmd.run(&mock_context);
	assert_eq!(
		result.err().unwrap().to_string(),
		"my_cold.vkey is not a valid Cardano key file: expected ident at line 1 column 2"
	);
}

/// Read the chain configuration file twice, because for some reason cardano network is not present in ChainConfig.
fn read_config_twice_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
		MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
	])
}

fn print_info_io() -> MockIO {
	MockIO::print(
		r##"This wizard will remove the specified candidate from the committee candidates based on the following chain parameters:
{
  "chain_id": 1234,
  "genesis_committee_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0",
  "threshold_numerator": 2,
  "threshold_denominator": 3,
  "governance_authority": "0x000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
}.
Committee Candidate Validator Address is 'addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc'
"##,
	)
}

fn read_keys_io() -> MockIO {
	MockIO::Group(vec![
        MockIO::print("Payment signing key and verification key of cold key used for registration are required to deregister."),
        read_payment_signing_key(),
        read_cold_verification_key(),
	])
}

fn read_payment_signing_key() -> MockIO {
	MockIO::Group(vec![
		MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
		MockIO::prompt(
			"path to the payment signing key file",
			Some("payment.skey"),
			MY_PAYMEMENT_SKEY,
		),
		MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
		MockIO::file_write_json_contains(
			RESOURCES_CONFIG_FILE_PATH,
			"/cardano_payment_signing_key_file",
			MY_PAYMEMENT_SKEY,
		),
		MockIO::file_read(MY_PAYMEMENT_SKEY),
	])
}

fn read_cold_verification_key() -> MockIO {
	MockIO::Group(vec![
		MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
		MockIO::prompt("path to the cold verification key file", Some("cold.vkey"), MY_COLD_VKEY),
		MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
		MockIO::file_write_json_contains(
			RESOURCES_CONFIG_FILE_PATH,
			"/cardano_cold_verification_key_file",
			MY_COLD_VKEY,
		),
		MockIO::file_read(MY_COLD_VKEY),
	])
}

fn run_smart_contract_io(output: &str) -> MockIO {
	MockIO::run_command("./pc-contracts-cli deregister --network testnet --ada-based-staking --spo-public-key 1111111111111111111111111111111111111111111111111111111111111111 --sidechain-id 1234 --genesis-committee-hash-utxo 0000000000000000000000000000000000000000000000000000000000000000#0 --threshold-numerator 2 --threshold-denominator 3 --governance-authority 0x000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9 --atms-kind plain-ecdsa-secp256k1 --kupo-host localhost --kupo-port 1442  --ogmios-host localhost --ogmios-port 1337  --payment-signing-key-file my_payment.skey", output)
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
			"network": 1
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
				"illiquid_supply_address": "addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz"
			},
		},
		"initial_permissioned_candidates": []
	})
}

fn invalid_chain_config_content() -> serde_json::Value {
	// Most of the required fields are missing
	json!({
		"cardano": {
			"security_parameter": 1234,
			"active_slots_coeff": 0.1,
			"first_epoch_timestamp_millis": 1_666_742_400_000_i64,
			"epoch_duration_millis": 86400000,
			"first_epoch_number": 1,
			"first_slot_number": 4320,
			"network": 1
		},
	})
}

fn chain_parameters_json() -> serde_json::Value {
	json!({
	  "chain_id": 1234,
	  "governance_authority": "0x000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9",
	  "threshold_numerator": 2,
	  "threshold_denominator": 3,
	  "genesis_committee_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0"
	})
}

fn test_resources_config_content() -> serde_json::Value {
	json!({
		"substrate_node_executable_path": "./partner-chains-node"
	})
}

fn valid_payment_signing_key_content() -> serde_json::Value {
	json!(
		{
		"type": "PaymentSigningKeyShelley_ed25519",
		"description": "Payment Signing Key",
		"cborHex": "58200000000000000000000000000000000000000000000000000000000000000001"
	})
}

fn valid_cold_verification_key_content() -> serde_json::Value {
	json!(
		{
			"type": "StakePoolVerificationKey_ed25519",
			"description": "Stake Pool Operator Verification Key",
			"cborHex": "58201111111111111111111111111111111111111111111111111111111111111111"
		}
	)
}
