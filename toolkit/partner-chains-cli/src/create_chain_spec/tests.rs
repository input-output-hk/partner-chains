use super::PartnerChainRuntime;
use crate::config::CHAIN_CONFIG_FILE_PATH;
use crate::create_chain_spec::{CreateChainSpecCmd, INITIAL_PERMISSIONED_CANDIDATES_EXAMPLE};
use crate::tests::{MockIO, MockIOContext};
use crate::{CmdRun, verify_json};
use anyhow::anyhow;
use colored::Colorize;
use pallet_session_validator_management::CommitteeMember;
use serde::Serialize;
use sp_core::{ecdsa, ed25519, sr25519};

#[derive(Debug, Default, Clone, Serialize)]
struct TestSessionKeys {
	aura: sr25519::Public,
	grandpa: ed25519::Public,
}

impl From<(sr25519::Public, ed25519::Public)> for TestSessionKeys {
	fn from((aura, grandpa): (sr25519::Public, ed25519::Public)) -> Self {
		Self { aura: aura.into(), grandpa: grandpa.into() }
	}
}

#[derive(Debug, Clone, Serialize)]
enum TestMember {
	Permissioned { id: ecdsa::Public, keys: TestSessionKeys },
}

impl CommitteeMember for TestMember {
	type AuthorityId = ecdsa::Public;
	type AuthorityKeys = TestSessionKeys;

	fn authority_id(&self) -> Self::AuthorityId {
		unimplemented!("unused")
	}
	fn authority_keys(&self) -> Self::AuthorityKeys {
		unimplemented!("unused")
	}
}

#[derive(Debug, Default, Clone)]
struct MockRuntime;
impl PartnerChainRuntime for MockRuntime {
	type AuthorityKeys = TestSessionKeys;
	type AuthorityId = ecdsa::Public;
	type CommitteeMember = TestMember;
	fn initial_member(id: Self::AuthorityId, keys: Self::AuthorityKeys) -> Self::CommitteeMember {
		TestMember::Permissioned { id, keys }
	}
}

fn create_chain_spec_cmd() -> CreateChainSpecCmd<MockRuntime> {
	CreateChainSpecCmd::default()
}

#[test]
fn happy_path() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_config_content())
		.with_json_file("chain-spec.json", test_chain_spec_content())
		.with_expected_io(vec![
			show_intro(),
			show_chain_parameters(),
			show_initial_permissioned_candidates(),
			MockIO::prompt_yes_no("Do you want to continue?", true, true),
			run_build_spec_io(Ok("ok".to_string())),
			show_outro(),
		]);
	let result = create_chain_spec_cmd().run(&mock_context);
	result.expect("should succeed");
	verify_json!(mock_context, "chain-spec.json", updated_chain_spec())
}

#[test]
fn shows_warning_when_initial_candidates_are_empty() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_config_content_with_empty_initial_permissioned_candidates())
		.with_expected_io(vec![
			show_intro(),
			show_chain_parameters(),
			MockIO::print(&"WARNING: The list of initial permissioned candidates is empty. Generated chain spec will not allow the chain to start.".red().to_string()),
			MockIO::print(&"Update 'initial_permissioned_candidates' field of pc-chain-config.json file with keys of initial committee.".red().to_string()),
			MockIO::print(&INITIAL_PERMISSIONED_CANDIDATES_EXAMPLE.yellow().to_string()),
			MockIO::prompt_yes_no("Do you want to continue?", true, false),
			MockIO::print("Aborted."),
		]
		);
	let result = create_chain_spec_cmd().run(&mock_context);
	assert!(result.is_ok());
}

#[test]
fn instruct_user_when_config_file_is_invalid() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_config_content_without_initial_permissioned_candidates())
		.with_expected_io(vec![
			MockIO::eprint("The 'pc-chain-config.json' configuration file is missing or invalid.
If you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.
If you are a validator, you can obtain the chain configuration file from the governance authority."),
		]);
	let result = create_chain_spec_cmd().run(&mock_context);
	result.expect_err("should return error");
}

#[test]
fn instruct_user_when_config_file_has_a_field_in_wrong_format() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_config_content_with_a_field_in_wrong_format())
		.with_expected_io(vec![
			MockIO::eprint("The 'pc-chain-config.json' configuration file is missing or invalid.
If you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.
If you are a validator, you can obtain the chain configuration file from the governance authority."),
		]);
	let result = create_chain_spec_cmd().run(&mock_context);
	result.expect_err("should return error");
}

#[test]
fn errors_if_chain_spec_is_missing() {
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_config_content())
		.with_expected_io(vec![
			show_intro(),
			show_chain_parameters(),
			show_initial_permissioned_candidates(),
			MockIO::prompt_yes_no("Do you want to continue?", true, true),
			run_build_spec_io(Ok("ok".to_string())),
		]);
	let result = create_chain_spec_cmd().run(&mock_context);
	let err = result.expect_err("should return error");
	assert_eq!(
		err.to_string(),
		"Could not read chain-spec.json file. File is expected to exists.".to_string()
	);
}

#[test]
fn forwards_build_spec_error_if_it_fails() {
	let error = anyhow!("Failed miserably".to_string());
	let mock_context = MockIOContext::new()
		.with_json_file(CHAIN_CONFIG_FILE_PATH, test_config_content())
		.with_json_file("chain-spec.json", test_chain_spec_content())
		.with_expected_io(vec![
			show_intro(),
			show_chain_parameters(),
			show_initial_permissioned_candidates(),
			MockIO::prompt_yes_no("Do you want to continue?", true, true),
			run_build_spec_io(Err(error)),
		]);
	let result = create_chain_spec_cmd().run(&mock_context);
	let err = result.expect_err("should return error");
	assert_eq!(err.to_string(), "Failed miserably".to_string());
}

fn test_config_content() -> serde_json::Value {
	serde_json::json!({
		"chain_parameters": chain_parameters_json(),
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
		"cardano_addresses": cardano_addresses_json(),
	})
}

fn test_config_content_with_a_field_in_wrong_format() -> serde_json::Value {
	serde_json::json!({
		"chain_parameters": chain_parameters_json(),
		"initial_permissioned_candidates": ["abc"]
	})
}

fn test_config_content_without_initial_permissioned_candidates() -> serde_json::Value {
	serde_json::json!({
		"chain_parameters": chain_parameters_json(),
	})
}

fn test_chain_spec_content() -> serde_json::Value {
	serde_json::json!(
		{
			"genesis": {
				"runtimeGenesis": {
					"config": {
						"session": {
							"initialValidators": []
						},
						"sessionCommitteeManagement": {
							"initialAuthorities": [],
							"main_chain_scripts": {
								"committee_candidate_address": "0x002244",
								"d_parameter_policy_id": "0x1234",
								"permissioned_candidates_policy_id": "0x5678"
							}
						},
						"governedMap": {
							"mainChainScripts": {
								"validator_address": "",
								"asset_policy_id": "0x0000"
							}
						}
					}
				}
			},
			"some_other_field": "irrelevant"
		}
	)
}

fn test_config_content_with_empty_initial_permissioned_candidates() -> serde_json::Value {
	serde_json::json!({
		"chain_parameters": chain_parameters_json(),
		"initial_permissioned_candidates": [],
		"cardano_addresses": cardano_addresses_json(),
	})
}

fn chain_parameters_json() -> serde_json::Value {
	serde_json::json!({
	  "genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0"
	})
}

fn cardano_addresses_json() -> serde_json::Value {
	serde_json::json!({
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
		"governed_map": {
			"validator_address": "addr_test1wqpjpjq08treyvmqjca0qy5kw8xgq4awgt945v46jsxgyhsafz4ws",
			"policy_id": "c814db91bfaf7f0078e2c69d13443ffc46c9957393174f7baa8d0000"
		}
	})
}

fn show_intro() -> MockIO {
	MockIO::Print("This wizard will create a chain spec JSON file according to the provided configuration, using WASM runtime code from the compiled node binary.".to_string())
}

fn show_chain_parameters() -> MockIO {
	MockIO::Group(vec![
		MockIO::print("Chain parameters:"),
		MockIO::print(
			"- Genesis UTXO: 0000000000000000000000000000000000000000000000000000000000000000#0",
		),
		MockIO::print("SessionValidatorManagement Main Chain Configuration:"),
		MockIO::print(
			"- committee_candidate_address: addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc",
		),
		MockIO::print(
			"- d_parameter_policy_id: d0ebb61e2ba362255a7c4a253c6578884603b56fb0a68642657602d6",
		),
		MockIO::print(
			"- permissioned_candidates_policy_id: 58b4ba68f641d58f7f1bba07182eca9386da1e88a34d47a14638c3fe",
		),
		MockIO::print("Native Token Management Configuration (unused if empty):"),
		MockIO::print("- asset name: 5043546f6b656e44656d6f"),
		MockIO::print(
			"- asset policy ID: ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
		),
		MockIO::print(
			"- illiquid supply address: addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz",
		),
		MockIO::print("Governed Map Configuration:"),
		MockIO::print(
			"- validator address: addr_test1wqpjpjq08treyvmqjca0qy5kw8xgq4awgt945v46jsxgyhsafz4ws",
		),
		MockIO::print(
			"- asset policy ID: c814db91bfaf7f0078e2c69d13443ffc46c9957393174f7baa8d0000",
		),
	])
}

fn show_initial_permissioned_candidates() -> MockIO {
	MockIO::Group(vec![
		MockIO::print("Initial permissioned candidates:"),
		MockIO::print(
			"- Partner Chains Key: 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1, AURA: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d, GRANDPA: 0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
		),
		MockIO::print(
			"- Partner Chains Key: 0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27, AURA: 0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48, GRANDPA: 0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69",
		),
	])
}

fn set_env_vars_io() -> MockIO {
	MockIO::Group(vec![
		MockIO::set_env_var(
			"GENESIS_UTXO",
			"0000000000000000000000000000000000000000000000000000000000000000#0",
		),
		MockIO::set_env_var(
			"COMMITTEE_CANDIDATE_ADDRESS",
			"addr_test1wz5qc7fk2pat0058w4zwvkw35ytptej3nuc3je2kgtan5dq3rt4sc",
		),
		MockIO::set_env_var(
			"D_PARAMETER_POLICY_ID",
			"d0ebb61e2ba362255a7c4a253c6578884603b56fb0a68642657602d6",
		),
		MockIO::set_env_var(
			"PERMISSIONED_CANDIDATES_POLICY_ID",
			"58b4ba68f641d58f7f1bba07182eca9386da1e88a34d47a14638c3fe",
		),
		MockIO::set_env_var(
			"NATIVE_TOKEN_POLICY_ID",
			"ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
		),
		MockIO::set_env_var("NATIVE_TOKEN_ASSET_NAME", "5043546f6b656e44656d6f"),
		MockIO::set_env_var(
			"ILLIQUID_SUPPLY_VALIDATOR_ADDRESS",
			"addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz",
		),
	])
}

fn run_build_spec_io(output: Result<String, anyhow::Error>) -> MockIO {
	MockIO::Group(vec![
		set_env_vars_io(),
		MockIO::RunCommand {
			expected_cmd:
				"<mock executable> build-spec --disable-default-bootnode > chain-spec.json"
					.to_string(),
			output,
		},
	])
}

fn updated_chain_spec() -> serde_json::Value {
	serde_json::json!(
		{
			"genesis": {
				"runtimeGenesis": {
					"config": {
						"session": {
							"initialValidators": [
								[
									  "5C7C2Z5sWbytvHpuLTvzKunnnRwQxft1jiqrLD5rhucQ5S9X",
									  {
										"aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
										"grandpa": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"
									}
								],
								[
									"5DVskgSC9ncWQpxFMeUn45NU43RUq93ByEge6ApbnLk6BR9N",
									{
										"aura": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
										"grandpa": "5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E"
									}
								]
							]
						},
						"sessionCommitteeManagement": {
							"initialAuthorities": [
								{
									"Permissioned":{
										"id": "KW39r9CJjAVzmkf9zQ4YDb2hqfAVGdRqn53eRqyruqpxAP5YL",
										"keys": {
											"aura": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
											"grandpa": "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu"
										}
									}
								},
								{
									"Permissioned":{
										"id": "KWByAN7WfZABWS5AoWqxriRmF5f2jnDqy3rB5pfHLGkY93ibN",
										"keys": {
											"aura": "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
											"grandpa": "5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E"
										}
									}
								}
							],
							"main_chain_scripts": {
								"committee_candidate_address": "0x002244",
								"d_parameter_policy_id": "0x1234",
								"permissioned_candidates_policy_id": "0x5678"
							}
						},
						"governedMap": {
							"mainChainScripts": {
								"asset_policy_id": "0xc814db91bfaf7f0078e2c69d13443ffc46c9957393174f7baa8d0000",
								"validator_address": "0x616464725f74657374317771706a706a71303874726579766d716a6361307179356b773878677134617767743934357634366a73786779687361667a347773"
							}
						}
					}
				}
			},
			"some_other_field": "irrelevant"
		}
	)
}

fn show_outro() -> MockIO {
	MockIO::Group(vec![
		MockIO::print("chain-spec.json file has been created."),
		MockIO::print(
			"If you are the governance authority, you can distribute it to the validators.",
		),
	])
}
