use crate::deregister::DeregisterCmd;
use crate::ogmios::config::tests::{
	default_ogmios_config_json, default_ogmios_service_config, establish_ogmios_configuration_io,
};
use crate::tests::{
	CHAIN_CONFIG_FILE_PATH, MockIO, MockIOContext, OffchainMock, OffchainMocks,
	RESOURCES_CONFIG_FILE_PATH,
};
use crate::{CmdRun, CommonArguments, verify_json};
use hex_literal::hex;
use serde_json::json;
use sidechain_domain::*;

const MY_PAYMEMENT_SKEY: &str = "my_payment.skey";
const MY_COLD_VKEY: &str = "my_cold.vkey";

#[test]
fn happy_path() {
	let offchain_mock = OffchainMock::new().with_deregister(
		genesis_utxo(),
		payment_signing_key(),
		stake_ownership_pub_key(),
		Ok(Some(McTxHash(hex!(
			"9aebb6d1d7f92f773f7d3025dd2fca67804ad6aea4a84a7696cd5ad15a4ee432"
		)))),
	);
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file(MY_PAYMEMENT_SKEY, valid_payment_signing_key_content())
		.with_json_file(MY_COLD_VKEY, valid_cold_verification_key_content())
		.with_offchain_mocks(OffchainMocks::new_with_mock("http://localhost:1337", offchain_mock))
		.with_expected_io(vec![
			print_info_io(),
			read_keys_io(),
			establish_ogmios_configuration_io(None, default_ogmios_service_config()),
		]);
	let result = deregister_cmd().run(&mock_context);
	assert!(result.is_ok());
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, final_resources_config_content());
}

#[test]
fn errors_if_smart_contracts_dont_output_transaction_id() {
	let offchain_mock = OffchainMock::new().with_deregister(
		genesis_utxo(),
		payment_signing_key(),
		stake_ownership_pub_key(),
		Err("test error".to_string()),
	);
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file(MY_PAYMEMENT_SKEY, valid_payment_signing_key_content())
		.with_json_file(MY_COLD_VKEY, valid_cold_verification_key_content())
		.with_offchain_mocks(OffchainMocks::new_with_mock("http://localhost:1337", offchain_mock))
		.with_expected_io(vec![
			print_info_io(),
			read_keys_io(),
			establish_ogmios_configuration_io(None, default_ogmios_service_config()),
		]);
	let result = deregister_cmd().run(&mock_context);
	assert_eq!(
		result.err().unwrap().to_string(),
		r#"Candidate deregistration failed: "test error"!"#
	);
	verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, final_resources_config_content());
}

#[test]
fn fails_when_chain_config_is_not_valid() {
	let mock_context =
		MockIOContext::new().with_json_file(CHAIN_CONFIG_FILE_PATH, invalid_chain_config_content());
	let result = deregister_cmd().run(&mock_context);
	assert_eq!(
		result.err().unwrap().to_string(),
		"Couldn't parse chain configuration file test-pc-chain-config.json. The chain configuration file that was used for registration is required in the working directory."
	);
}

#[test]
fn fails_when_payment_signing_key_is_not_valid() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_file(MY_PAYMEMENT_SKEY, "not a proper Cardano key json")
		.with_expected_io(vec![
			print_info_io(),
            MockIO::print("Payment signing key and cold verification key used for registration are required to deregister."),
            read_payment_signing_key()
		]);
	let result = deregister_cmd().run(&mock_context);
	assert_eq!(
		result.err().unwrap().to_string(),
		"Failed to parse Cardano key file my_payment.skey: 'expected ident at line 1 column 2'"
	);
}

#[test]
fn fails_when_cold_key_is_not_valid() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_chain_config_content())
		.with_json_file(RESOURCES_CONFIG_FILE_PATH, test_resources_config_content())
		.with_json_file(MY_PAYMEMENT_SKEY, valid_payment_signing_key_content())
		.with_file(MY_COLD_VKEY, "not a proper Cardano key json")
		.with_expected_io(vec![print_info_io(), read_keys_io()]);
	let result = deregister_cmd().run(&mock_context);
	assert_eq!(
		result.err().unwrap().to_string(),
		"Failed to parse Cardano key file my_cold.vkey: 'expected ident at line 1 column 2'"
	);
	verify_json!(
		mock_context,
		RESOURCES_CONFIG_FILE_PATH,
		json!({"cardano_payment_signing_key_file": MY_PAYMEMENT_SKEY, "cardano_cold_verification_key_file": MY_COLD_VKEY})
	);
}

fn deregister_cmd() -> DeregisterCmd {
	DeregisterCmd { common_arguments: CommonArguments { retry_delay_seconds: 5, retry_count: 59 } }
}

fn print_info_io() -> MockIO {
	MockIO::print(
		r##"This wizard will remove the specified candidate from the committee candidates based on the following chain parameters:
{
  "genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0"
}.
Committee Candidate Validator Address is 'addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc'
"##,
	)
}

fn read_keys_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::print(
			"Payment signing key and cold verification key used for registration are required to deregister.",
		),
		read_payment_signing_key(),
		read_cold_verification_key(),
	])
}

fn read_payment_signing_key() -> MockIO {
	MockIO::prompt(
		"Enter the path to the payment signing key file",
		Some("payment.skey"),
		MY_PAYMEMENT_SKEY,
	)
}

fn read_cold_verification_key() -> MockIO {
	MockIO::prompt(
		"Enter the path to the cold verification key file",
		Some("cold.vkey"),
		MY_COLD_VKEY,
	)
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
				"illiquid_circulation_supply_validator_address": "addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz"
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
			"network": "testnet"
		},
	})
}

fn chain_parameters_json() -> serde_json::Value {
	json!({
	  "genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0"
	})
}

fn test_resources_config_content() -> serde_json::Value {
	json!({})
}

fn final_resources_config_content() -> serde_json::Value {
	json!({
		"cardano_payment_signing_key_file": MY_PAYMEMENT_SKEY,
		"cardano_cold_verification_key_file": MY_COLD_VKEY,
		"ogmios": default_ogmios_config_json()
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

fn genesis_utxo() -> UtxoId {
	"0000000000000000000000000000000000000000000000000000000000000000#0"
		.parse()
		.unwrap()
}

fn payment_signing_key() -> Vec<u8> {
	hex!("0000000000000000000000000000000000000000000000000000000000000001").to_vec()
}

fn stake_ownership_pub_key() -> StakePoolPublicKey {
	StakePoolPublicKey(hex!("1111111111111111111111111111111111111111111111111111111111111111"))
}
