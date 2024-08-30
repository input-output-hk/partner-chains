use crate::config::config_fields::{
	CARDANO_NETWORK, COMMITTEE_CANDIDATES_ADDRESS, D_PARAMETER_POLICY_ID, ILLIQUID_SUPPLY_ADDRESS,
	INITIAL_PERMISSIONED_CANDIDATES, NATIVE_TOKEN_ASSET_NAME, NATIVE_TOKEN_POLICY,
	PERMISSIONED_CANDIDATES_POLICY_ID,
};
use crate::config::{
	get_cardano_network_from_file, CardanoNetwork, SidechainParams, PC_CONTRACTS_CLI_PATH,
};
use crate::io::IOContext;
use crate::pc_contracts_cli_resources::{
	establish_pc_contracts_cli_configuration, PcContractsCliResources,
};
use crate::prepare_configuration::prepare_cardano_params::prepare_cardano_params;
use crate::smart_contracts;
use anyhow::anyhow;
use serde_json::Value;
use sidechain_domain::PolicyId;

// pc-contracts-cli addresses command requires providing a signing key, but the key has no influence on
// its output, thus using a dummy key
const DUMMY_SKEY: &str = "{
	\"type\": \"PaymentSigningKeyShelley_ed25519\",
	\"description\": \"Payment Signing Key\",
	\"cborHex\": \"58200000000000000000000000000000000000000000000000000000000000000000\"
}";

pub fn prepare_main_chain_config<C: IOContext>(
	context: &C,
	sidechain_params: SidechainParams,
) -> anyhow::Result<()> {
	if !context.file_exists(PC_CONTRACTS_CLI_PATH) {
		return Err(anyhow!(
			"Partner Chains Smart Contracts executable file ({}) is missing",
			PC_CONTRACTS_CLI_PATH
		));
	}
	let pc_contracts_cli_version = context.run_command(PC_CONTRACTS_CLI_VERSION_CMD)?;
	context.eprint(&pc_contracts_cli_version_prompt(pc_contracts_cli_version));

	let cardano_network = prompt_cardano_network(context)?;
	prepare_cardano_params(context, cardano_network)?;
	set_up_cardano_addresses(context, sidechain_params)?;
	if INITIAL_PERMISSIONED_CANDIDATES.load_from_file(context).is_none() {
		INITIAL_PERMISSIONED_CANDIDATES.save_to_file(&vec![], context)
	}
	prepare_native_token(context)?;
	context.eprint(OUTRO);
	Ok(())
}

fn prompt_for_custom_cardano_network_id(context: &impl IOContext) -> anyhow::Result<u32> {
	loop {
		let id = context.prompt("Enter custom cardano network ID", Some("3"));
		match id.parse::<u32>() {
			Ok(id) if id >= 3 => return Ok(id),
			_ => context.eprint("Custom cardano network ID must be a number greater or equal to 3"),
		}
	}
}

fn prompt_cardano_network<C: IOContext>(context: &C) -> anyhow::Result<u32> {
	let selected_network: CardanoNetwork = CARDANO_NETWORK
		.select_options_with_default_from_file_and_save(CHOOSE_CARDANO_NETWORK, context)
		.map_err(anyhow::Error::msg)?;
	let cardano_network: CardanoNetwork = match selected_network {
		CardanoNetwork(id) if id >= 3 => {
			let custom_id = prompt_for_custom_cardano_network_id(context)?;
			CardanoNetwork(custom_id)
		},
		_ => selected_network,
	};
	CARDANO_NETWORK.save_to_file(&cardano_network, context);
	Ok(cardano_network.to_id())
}

fn set_up_cardano_addresses<C: IOContext>(
	context: &C,
	params: SidechainParams,
) -> anyhow::Result<()> {
	let kupo_ogmios_config = establish_pc_contracts_cli_configuration(context)?;
	run_pc_contracts_cli_addresses(context, params, kupo_ogmios_config)?;
	Ok(())
}

fn run_pc_contracts_cli_addresses<C: IOContext>(
	context: &C,
	params: SidechainParams,
	kupo_and_ogmios_config: PcContractsCliResources,
) -> anyhow::Result<()> {
	let dummy_key_file = context.new_tmp_file(DUMMY_SKEY);
	let cardano_network = get_cardano_network_from_file(context)?;
	let cmd = addresses_cmd(
		dummy_key_file
			.to_str()
			.ok_or(anyhow!("Cannot convert temporary file name to unicode string"))?
			.to_string(),
		params,
		&kupo_and_ogmios_config,
		cardano_network,
	);
	let addresses_string = context.run_command(&cmd)?;

	let addresses_json: Value = serde_json::from_str(&addresses_string).map_err(|_| {
		anyhow!("Failed to fetch data from Ogmios or Kupo. Please check connection configuration and try again.")
	})?;

	COMMITTEE_CANDIDATES_ADDRESS.save_to_file(
		&addresses_json.pointer("/addresses/CommitteeCandidateValidator")
			.ok_or(anyhow!("committee candidate address missing from pc-contracts-cli addresses command output"))?
			.as_str()
			.ok_or(anyhow!("committee candidate address from pc-contracts-cli addresses command output cannot be converted to string"))?
			.to_string(),
		context);
	D_PARAMETER_POLICY_ID.save_to_file(
		&addresses_json
			.pointer("/mintingPolicies/DParameterPolicy")
			.ok_or(anyhow!(
				"D parameter policy id missing from pc-contracts-cli addresses command output"
			))?
			.as_str()
			.ok_or(anyhow!("D parameter policy id from pc-contracts-cli addresses command output cannot be converted to string"))?
			.to_string(),
		context,
	);
	PERMISSIONED_CANDIDATES_POLICY_ID.save_to_file(
		&addresses_json.pointer("/mintingPolicies/PermissionedCandidatesPolicy")
			.ok_or(anyhow!("permissioned candidates policy id address missing from pc-contracts-cli addresses command output"))?
			.as_str()
			.ok_or(anyhow!("Permissioned candidates policy id from pc-contracts-cli addresses command output cannot be converted to string"))?
			.to_string(),
		context);
	ILLIQUID_SUPPLY_ADDRESS.save_to_file(
		&addresses_json.pointer("/addresses/IlliquidCirculationSupplyValidator")
			.ok_or(anyhow!("Illiquid circulation supply validator address is missing from pc-contracts-cli addresses command output"))?
			.as_str()
			.ok_or(anyhow!("Illiquid circulation supply validator address from pc-contracts-cli addresses command output cannot be converted to string"))?
			.to_string(),
		context,
	);
	Ok(())
}

fn prepare_native_token<C: IOContext>(context: &C) -> anyhow::Result<()> {
	context.print(
		"Partner Chains can store their initial token supply on Cardano as Cardano native tokens.",
	);
	context.print("Creation of the native token is not supported by this wizard and must be performed manually before this step.");
	if context.prompt_yes_no("Do you want to configure a native token for you Partner Chain?", true)
	{
		NATIVE_TOKEN_POLICY.prompt_with_default_from_file_and_save(context);
		NATIVE_TOKEN_ASSET_NAME.prompt_with_default_from_file_and_save(context);
	} else {
		NATIVE_TOKEN_POLICY.save_to_file(&PolicyId::default().to_hex_string(), context);
		NATIVE_TOKEN_ASSET_NAME.save_to_file(&"0x0".into(), context);
	}

	Ok(())
}

fn addresses_cmd(
	key_file_path: String,
	params: SidechainParams,
	kupo_and_ogmios_config: &PcContractsCliResources,
	network: CardanoNetwork,
) -> String {
	let sidechain_param_arg = smart_contracts::sidechain_params_arguments(&params);
	format!(
		"{PC_CONTRACTS_CLI_PATH} addresses \
	--network {} \
	{} \
	--version 1 \
	--payment-signing-key-file {} \
    --kupo-host {} \
    --kupo-port {} \
    {} \
    --ogmios-host {} \
    --ogmios-port {} \
    {} \
	",
		network.to_network_param(),
		sidechain_param_arg,
		key_file_path,
		kupo_and_ogmios_config.kupo.hostname,
		kupo_and_ogmios_config.kupo.port,
		if kupo_and_ogmios_config.kupo.protocol.is_secure() { "--kupo-secure" } else { "" },
		kupo_and_ogmios_config.ogmios.hostname,
		kupo_and_ogmios_config.ogmios.port,
		if kupo_and_ogmios_config.ogmios.protocol.is_secure() { "--ogmios-secure" } else { "" },
	)
}

const CHOOSE_CARDANO_NETWORK: &str = "Which cardano network would you like to use?";
const OUTRO: &str = r#"Chain configuration (partner-chains-cli-chain-config.json) is now ready for distribution to network participants.

If you intend to run a chain with permissioned candidates, you must manually set their keys in the partner-chains-cli-chain-config.json file before proceeding. Here's an example of how to add permissioned candidates:

{
  ...
  "initial_permissioned_candidates": [
    {
      "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde49a5684e7a56da27d",
      "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200f498922423d4334014fa6b0ee",
      "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e0477968906ac916b04cb365ec3153755684d9a1"
    },
    {
      "aura_pub_key": "0x8eaf04151687736326c9fea17e25fc5287613698c912909cb226aa4794f26a48",
      "grandpa_pub_key": "0xd17c2d7823ebf260fd138f2d7e27d114cb145d968b5ff5006125f2414fadae69",
      "sidechain_pub_key": "0x0390084fdbf27d2b79d26a4f13f0cdd982cb755a661969143c37cbc49ef5b91f27"
    }
  ]
}

After setting up the permissioned candidates, execute the 'create-chain-spec' command to generate the final chain specification."#;

const PC_CONTRACTS_CLI_VERSION_CMD: &str = "./pc-contracts-cli cli-version";

fn pc_contracts_cli_version_prompt(version: String) -> String {
	format!("{} {}", PC_CONTRACTS_CLI_PATH, version)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::config_fields::{GOVERNANCE_AUTHORITY, KUPO_PROTOCOL};
	use crate::config::CHAIN_CONFIG_FILE_PATH;
	use crate::prepare_configuration::prepare_cardano_params::PREPROD_CARDANO_PARAMS;
	use crate::prepare_configuration::tests::save_to_existing_file;
	use crate::tests::MockIO;
	use crate::tests::MockIOContext;
	use serde_json::Value;
	use sidechain_domain::{MainchainAddressHash, UtxoId};
	use std::str::FromStr;

	const TEST_GENESIS_UTXO: &str =
		"0000000000000000000000000000000000000000000000000000000000000000#0";
	const TEST_D_PARAMETER_POLICY_ID: &str =
		"623cc9d41321674962b8599bf2baf0f34b8df8ad9d549f7ba3b1fdbb";
	const TEST_COMMITTEE_CANDIDATES_ADDRESS: &str =
		"addr_test1wz5fe8fmxx4v83gzfsdlnhgxm8x7zpldegrqh2wakl3wteqe834r4";
	const TEST_PERMISSIONED_CANDIDATES_POLICY_ID: &str =
		"13db1ba564b3b264f45974fece44b2beb0a2326b10e65a0f7f300dfb";
	const TEST_ILLIQUID_SUPPLY_ADDRESS: &str =
		"addr_test1wqn2pkvvmesmxtfa4tz7w8gh8vumr52lpkrhcs4dkg30uqq77h5z4";
	const PC_CONTRACTS_CLI_VERSION_CMD_OUTPUT: &str =
		"Version: 5.0.0, a770e9bbdcc9410575f8d47c0890801b4ae5c31a";
	const PC_CONTRACTS_CLI: &str = "./pc-contracts-cli";

	pub mod scenarios {
		use super::*;
		use crate::config::config_fields::CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS;
		use crate::prepare_configuration::prepare_cardano_params::tests::scenarios::save_cardano_params_but_last;

		pub fn save_cardano_params() -> MockIO {
			MockIO::Group(vec![
				save_cardano_params_but_last(PREPROD_CARDANO_PARAMS),
				save_to_existing_file(
					CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS,
					&PREPROD_CARDANO_PARAMS.first_epoch_timestamp_millis.to_string(),
				),
			])
		}

		pub fn prompt_and_save_native_asset_scripts() -> MockIO {
			MockIO::Group(vec![
						MockIO::print("Partner Chains can store their initial token supply on Cardano as Cardano native tokens."),
						MockIO::print("Creation of the native token is not supported by this wizard and must be performed manually before this step."),
						MockIO::prompt_yes_no(
							"Do you want to configure a native token for you Partner Chain?",
							true,
							true,
						),
						MockIO::file_read(NATIVE_TOKEN_POLICY.config_file),
						MockIO::prompt(
							NATIVE_TOKEN_POLICY.name,
							None,
							"ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
						),
						MockIO::file_read(NATIVE_TOKEN_POLICY.config_file),
						MockIO::file_write_json_contains(
							NATIVE_TOKEN_POLICY.config_file,
							&NATIVE_TOKEN_POLICY.json_pointer(),
							"ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
						),
						MockIO::file_read(NATIVE_TOKEN_ASSET_NAME.config_file),
						MockIO::prompt(NATIVE_TOKEN_ASSET_NAME.name, None, "5043546f6b656e44656d6f"),
						MockIO::file_read(NATIVE_TOKEN_ASSET_NAME.config_file),
						MockIO::file_write_json_contains(
							NATIVE_TOKEN_ASSET_NAME.config_file,
							&NATIVE_TOKEN_ASSET_NAME.json_pointer(),
							"5043546f6b656e44656d6f",
						),
					])
		}
	}

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_file(PC_CONTRACTS_CLI, "<mock executable>")
			.with_json_file(GOVERNANCE_AUTHORITY.config_file, serde_json::json!({}))
			.with_json_file(KUPO_PROTOCOL.config_file, serde_json::json!({}))
			.with_expected_io(vec![
				MockIO::run_command(PC_CONTRACTS_CLI_VERSION_CMD, PC_CONTRACTS_CLI_VERSION_CMD_OUTPUT),
				MockIO::eprint(&pc_contracts_cli_version_prompt(PC_CONTRACTS_CLI_VERSION_CMD_OUTPUT.to_string())),

				MockIO::file_read(CARDANO_NETWORK.config_file),
				MockIO::prompt_multi_option(
					CHOOSE_CARDANO_NETWORK,
					vec!["mainnet".to_string(), "preprod".to_string(), "preview".to_string(), "custom".to_string()],
					"preprod"
				),
				MockIO::file_read(CARDANO_NETWORK.config_file),
				MockIO::file_write_json_contains(
					CARDANO_NETWORK.config_file,
					&CARDANO_NETWORK.json_pointer(),
					"1",
				),
				MockIO::file_read(CARDANO_NETWORK.config_file),
				MockIO::file_write_json_contains(
					CARDANO_NETWORK.config_file,
					&CARDANO_NETWORK.json_pointer(),
					"1",
				),
				scenarios::save_cardano_params(),
				crate::pc_contracts_cli_resources::tests::establish_pc_contracts_cli_configuration_io(None, PcContractsCliResources::default()),
				MockIO::new_tmp_file(DUMMY_SKEY),
				MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
				MockIO::run_command(
					&addresses_cmd(
						"/tmp/dummy3".to_string(),
						test_sidechain_params(),
						&PcContractsCliResources::default(),
						CardanoNetwork(1)
					),
					&test_addresses_cmd_output().to_string(),
				),
				save_to_existing_file(
					COMMITTEE_CANDIDATES_ADDRESS,
					TEST_COMMITTEE_CANDIDATES_ADDRESS,
				),
				save_to_existing_file(D_PARAMETER_POLICY_ID, TEST_D_PARAMETER_POLICY_ID),
				save_to_existing_file(
					PERMISSIONED_CANDIDATES_POLICY_ID,
					TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
				),
				save_to_existing_file(
					ILLIQUID_SUPPLY_ADDRESS,
					TEST_ILLIQUID_SUPPLY_ADDRESS,
				),
				MockIO::file_read(INITIAL_PERMISSIONED_CANDIDATES.config_file),
				MockIO::file_read(INITIAL_PERMISSIONED_CANDIDATES.config_file),
				MockIO::file_write_json(
					INITIAL_PERMISSIONED_CANDIDATES.config_file,
					test_chain_config(),
				),

				scenarios::prompt_and_save_native_asset_scripts(),

				MockIO::eprint(OUTRO),
			]);
		prepare_main_chain_config(&mock_context, test_sidechain_params()).expect("should succeed");
	}

	#[test]
	fn happy_path_with_initial_permissioned_candidates() {
		let mock_context = MockIOContext::new()
			.with_file(PC_CONTRACTS_CLI, "<mock executable>")
			.with_json_file(INITIAL_PERMISSIONED_CANDIDATES.config_file, serde_json::json!({
				"initial_permissioned_candidates": [
					{
					  "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
					  "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
					  "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"
					}
				]
			}))
			.with_json_file(KUPO_PROTOCOL.config_file, serde_json::json!({}))
			.with_expected_io(vec![
				MockIO::run_command(PC_CONTRACTS_CLI_VERSION_CMD, PC_CONTRACTS_CLI_VERSION_CMD_OUTPUT),
				MockIO::eprint(&pc_contracts_cli_version_prompt(PC_CONTRACTS_CLI_VERSION_CMD_OUTPUT.to_string())),
				MockIO::file_read(CARDANO_NETWORK.config_file),
				MockIO::prompt_multi_option(
					CHOOSE_CARDANO_NETWORK,
					vec!["mainnet".to_string(), "preprod".to_string(), "preview".to_string(), "custom".to_string()],
					"preprod"
				),
				MockIO::file_read(CARDANO_NETWORK.config_file),
				MockIO::file_write_json_contains(
					CARDANO_NETWORK.config_file,
					&CARDANO_NETWORK.json_pointer(),
					"1",
				),
				MockIO::file_read(CARDANO_NETWORK.config_file),
				MockIO::file_write_json_contains(
					CARDANO_NETWORK.config_file,
					&CARDANO_NETWORK.json_pointer(),
					"1",
				),
				scenarios::save_cardano_params(),
				crate::pc_contracts_cli_resources::tests::establish_pc_contracts_cli_configuration_io(None, PcContractsCliResources::default()),
				MockIO::new_tmp_file(DUMMY_SKEY),
				MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
				MockIO::run_command(
					&addresses_cmd(
						"/tmp/dummy3".to_string(),
						test_sidechain_params(),
						&PcContractsCliResources::default(),
						CardanoNetwork(1)
					),
					&test_addresses_cmd_output().to_string(),
				),
				save_to_existing_file(
					COMMITTEE_CANDIDATES_ADDRESS,
					TEST_COMMITTEE_CANDIDATES_ADDRESS,
				),
				save_to_existing_file(D_PARAMETER_POLICY_ID, TEST_D_PARAMETER_POLICY_ID),
				save_to_existing_file(
					PERMISSIONED_CANDIDATES_POLICY_ID,
					TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
				),
				save_to_existing_file(
					ILLIQUID_SUPPLY_ADDRESS,
					TEST_ILLIQUID_SUPPLY_ADDRESS,
				),
				MockIO::file_read(INITIAL_PERMISSIONED_CANDIDATES.config_file),
				scenarios::prompt_and_save_native_asset_scripts(),
				MockIO::eprint(OUTRO),
			]);
		prepare_main_chain_config(&mock_context, test_sidechain_params()).expect("should succeed");
	}

	#[test]
	fn should_return_relevant_error_when_partner_chains_scripts_executable_missing() {
		let context = MockIOContext::new();
		let result = prepare_main_chain_config(&context, test_sidechain_params());

		assert!(result.is_err());
		assert_eq!(
			result.unwrap_err().to_string(),
			"Partner Chains Smart Contracts executable file (./pc-contracts-cli) is missing"
		)
	}

	fn test_sidechain_params() -> SidechainParams {
		SidechainParams {
			chain_id: 0,
			genesis_committee_utxo: UtxoId::from_str(TEST_GENESIS_UTXO).unwrap(),
			threshold_numerator: 2,
			threshold_denominator: 3,
			governance_authority: MainchainAddressHash::from_hex_unsafe(
				"0x76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9",
			),
		}
	}

	fn test_chain_config() -> Value {
		serde_json::json!({
			"cardano": {
				"network": 1,
				"security_parameter": PREPROD_CARDANO_PARAMS.security_parameter,
				"active_slots_coeff": PREPROD_CARDANO_PARAMS.active_slots_coeff,
				"first_epoch_number": PREPROD_CARDANO_PARAMS.first_epoch_number,
				"first_slot_number": PREPROD_CARDANO_PARAMS.first_slot_number,
				"epoch_duration_millis": PREPROD_CARDANO_PARAMS.epoch_duration_millis,
				"first_epoch_timestamp_millis": PREPROD_CARDANO_PARAMS.first_epoch_timestamp_millis
			},
			"cardano_addresses": {
				"committee_candidates_address": TEST_COMMITTEE_CANDIDATES_ADDRESS,
				"d_parameter_policy_id": TEST_D_PARAMETER_POLICY_ID,
				"permissioned_candidates_policy_id": TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
				"native_token": {
					"illiquid_supply_address": TEST_ILLIQUID_SUPPLY_ADDRESS,
				}
			},
			"initial_permissioned_candidates": []
		})
	}

	fn test_addresses_cmd_output() -> Value {
		serde_json::json!({
		  "endpoint": "GetAddrs",
		  "addresses": {
			"CommitteeCandidateValidator": TEST_COMMITTEE_CANDIDATES_ADDRESS,
			"DsConfValidator": "addr_test1wrmqwktean5p7j8k5nytx22lhu49rr7u8dgnr3erea3havqqjajlv",
			"DsInsertValidator": "addr_test1wrvnmyz9e6gaxpp3tvqs6hrudnvt53pufec27t3ljp4m0cq2dme0x",
			"VersionOracleValidator": "addr_test1wr9cypuct5mduqqx7qvga2pmp5p0xpdsxju3k7xre5xsj6c6sgeeg",
			"PermissionedCandidatesValidator": "addr_test1wzw9j9wrz7jcf90uawyck69dq5s63juxwdy5ngjtd2hnfwcg8xc7u",
			"DParameterValidator": "addr_test1wpqqhw53gnqqpv0t694qlnafkvv2yl4fkusvq5lue9z4srqrqjdh3",
			"MerkleRootTokenValidator": "addr_test1wz3gsl0z2wsrqeav0mr869avzln5kuz5dl8wal4mlm6w5nsast866",
			"CheckpointValidator": "addr_test1wrln5wjm88f3sg7yg58nt4mlefr45qq89pg3yf6l39kavlqu5jnwk",
			"CommitteeHashValidator": "addr_test1wrpf0tq92fgwc3t3y2fe8pf7tgzuneehfh0p6hvp8y5kemsyrkaa5",
			"IlliquidCirculationSupplyValidator": TEST_ILLIQUID_SUPPLY_ADDRESS
		  },
		  "validatorHashes": {
			"CommitteeCandidateValidator": "a89c9d3b31aac3c5024c1bf9dd06d9cde107edca060ba9ddb7e2e5e4",
			"DsConfValidator": "f6075979ece81f48f6a4c8b3295fbf2a518fdc3b5131c723cf637eb0",
			"DsInsertValidator": "d93d9045ce91d304315b010d5c7c6cd8ba443c4e70af2e3f906bb7e0",
			"VersionOracleValidator": "cb8207985d36de0006f0188ea83b0d02f305b034b91b78c3cd0d096b",
			"PermissionedCandidatesValidator": "9c5915c317a58495fceb898b68ad0521a8cb86734949a24b6aaf34bb",
			"DParameterValidator": "400bba9144c000b1ebd16a0fcfa9b318a27ea9b720c053fcc945580c",
			"MerkleRootTokenValidator": "a2887de253a03067ac7ec67d17ac17e74b70546fceeefebbfef4ea4e",
			"CheckpointValidator": "ff3a3a5b39d31823c4450f35d77fca475a0007285112275f896dd67c",
			"CommitteeHashValidator": "c297ac055250ec4571229393853e5a05c9e7374dde1d5d8139296cee"
		  },
		  "mintingPolicies": {
			"DsConfPolicy": "d382159e47b8c0789d0c9a943ddc3801c616b52b03fefb807a2d86d2",
			"CheckpointPolicy": "138d5e4c8c22a96b344333b551c8c220b9db1bcbc3ae40b18d408dd2",
			"FUELProxyPolicy": "687f30b77dc773584db2786bca1d39c45635f9041697b7ebb479ad63",
			"VersionOraclePolicy": "3480f702b814d4a24ad7553e879cbc222478ecacb9c02fcb8d4ae92f",
			"PermissionedCandidatesPolicy": TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
			"DParameterPolicy": TEST_D_PARAMETER_POLICY_ID,
			"InitTokenPolicy": "3a0cdc0fa199367332576bc73bbc0ad3db1a6cc945df453e1fc54a31",
			"MerkleRootTokenPolicy": "9b7e2c884d86d00ee544927821dff8c94ff676fc5f0647efd869a960",
			"FUELMintingPolicy": "1af85db994c03112ba9374811b22f3100fa3153c1e5f9a48200020e9",
			"FUELBurningPolicy": "3c2de98c8da1006f3310cf19b73661e7a1f2d4eb3dc27c43f608aca2",
			"DsKeyPolicy": "91da86f628fa49c3499942215b833708424d47f72c44bdac73a0c217",
			"CommitteeCertificateVerificationPolicy": "3473f8cfde8fa047f68d67f9e22df0423a73b3accb668d9f486eebcb",
			"CommitteeOraclePolicy": "876c5fce084ced24066320ff63fc6add191207932c8bf14db7fda9ff",
			"CommitteePlainEcdsaSecp256k1ATMSPolicy": "3473f8cfde8fa047f68d67f9e22df0423a73b3accb668d9f486eebcb"
		  }
		})
	}
}
