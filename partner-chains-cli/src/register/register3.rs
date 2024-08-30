use crate::config;
use crate::config::config_fields;
use crate::config::get_cardano_network_from_file;
use crate::config::CHAIN_CONFIG_FILE_PATH;
use crate::config::PC_CONTRACTS_CLI_PATH;
use crate::io::IOContext;
use crate::main_chain_follower::set_main_chain_follower_env;
use crate::pc_contracts_cli_resources::establish_pc_contracts_cli_configuration;
use crate::smart_contracts;
use crate::CmdRun;
use clap::Parser;
use cli_commands::key_params::{PlainPublicKeyParam, SidechainPublicKeyParam};
use epoch_derivation::{MainchainEpochConfig, MainchainEpochDerivation};
use sidechain_domain::{McEpochNumber, UtxoId};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Register3Cmd {
	#[clap(flatten)]
	pub sidechain_params: config::SidechainParams,
	#[arg(long)]
	pub registration_utxo: UtxoId,
	#[arg(long)]
	pub sidechain_pub_key: SidechainPublicKeyParam,
	#[arg(long)]
	pub aura_pub_key: PlainPublicKeyParam,
	#[arg(long)]
	pub grandpa_pub_key: PlainPublicKeyParam,
	#[arg(long)]
	pub sidechain_signature: String,
	#[arg(long)]
	pub spo_public_key: String,
	#[arg(long)]
	pub spo_signature: String,
}

impl CmdRun for Register3Cmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.print("⚙️ Register as a committee candidate (step 3/3)");
		context.print("This command will submit the registration message to the mainchain.");

		config_fields::GOVERNANCE_AUTHORITY.load_from_file(context).ok_or_else(|| {
			context.eprint(&format!("⚠️ The chain configuration file `{CHAIN_CONFIG_FILE_PATH}` is missing or invalid.\n"));
			context.eprint("⚠️ If you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.\n");
			context.eprint("⚠️ If you are a validator, you can obtain the chain configuration file from the governance authority.\n");
			anyhow::anyhow!("Chain config missing or invalid")
		})?;

		context.print("To proceed with the next command, a payment signing key is required. Please note that this key will not be stored or communicated over the network.");

		let cardano_payment_signing_key_path =
			context.prompt("Path to mainchain payment signing key file", Some("payment.skey"));

		let sidechain_param_arg =
			smart_contracts::sidechain_params_arguments(&self.sidechain_params);

		let pc_contracts_cli_resources = establish_pc_contracts_cli_configuration(context)?;
		let runtime_config_arguments = smart_contracts::runtime_config_arguments(
			&pc_contracts_cli_resources,
			&cardano_payment_signing_key_path,
		);

		let cardano_network = get_cardano_network_from_file(context)?;

		let command = format!(
			"{PC_CONTRACTS_CLI_PATH} register --network {} {} --registration-utxo {} --sidechain-public-keys {}:{}:{} --sidechain-signature {} --spo-public-key {} --spo-signature {} --ada-based-staking {}",
			cardano_network.to_network_param(),
			sidechain_param_arg,
			self.registration_utxo,
			self.sidechain_pub_key,
			self.aura_pub_key,
			self.grandpa_pub_key,
			self.sidechain_signature,
			self.spo_public_key,
			self.spo_signature,
			runtime_config_arguments
		);

		let sidechain_cli_output = context.run_command(&command)?;
		error_if_registration_call_failed(context, sidechain_cli_output)?;

		if context.prompt_yes_no("Show registration status?", true) {
			context.print("The registration status will be queried from a db-sync instance for which a valid connection string is required. Please note that this db-sync instance needs to be up and synced with the main chain.");
			let current_mc_epoch_number = get_current_mainchain_epoch(context).map_err(|e| {
				context.eprint(format!("Unable to derive current mainchain epoch: {}", e).as_str());
				anyhow::anyhow!("{}", e)
			})?;
			let mc_epoch_number_to_query = current_mc_epoch_number.next().next();
			prepare_mc_follower_env(context)?;
			context.print(&format!("Registrations status for epoch {mc_epoch_number_to_query}:"));
			show_registration_status(
				context,
				mc_epoch_number_to_query,
				self.spo_public_key.clone(),
			)?;
		}

		Ok(())
	}
}

fn prepare_mc_follower_env<C: IOContext>(context: &C) -> anyhow::Result<()> {
	let postgres_connection_string =
		config_fields::POSTGRES_CONNECTION_STRING.prompt_with_default_from_file_and_save(context);
	let chain_config = config::load_chain_config(context)?;
	set_main_chain_follower_env(context, &chain_config.cardano, &postgres_connection_string);
	Ok(())
}

fn show_registration_status(
	context: &impl IOContext,
	mc_epoch_number: McEpochNumber,
	mc_public_key: String,
) -> Result<(), anyhow::Error> {
	let temp_dir = context.new_tmp_dir();
	let temp_dir_path = temp_dir
		.into_os_string()
		.into_string()
		.expect("PathBuf is a valid UTF-8 String");
	let node_executable = config_fields::NODE_EXECUTABLE
		.load_from_file(context)
		.ok_or_else(|| anyhow::anyhow!("⚠️ Unable to load node executable"))?;
	let command = format!(
		"{} registration-status --mainchain-pub-key {} --mc-epoch-number {} --chain chain-spec.json --base-path {temp_dir_path}",
		node_executable, mc_public_key, mc_epoch_number
	);
	let output = context.run_command(&command)?;
	context.print("Registration status:");
	context.print(&output);
	Ok(())
}

// An example of a succesful output:
// {"endpoint":"CommitteeCandidateReg","transactionId":"1ab93b52d20ce114bfdb48a256ac48f3d8d46d00aec585c38a904b672a70e3a3"}
fn error_if_registration_call_failed(
	context: &impl IOContext,
	output: String,
) -> Result<(), anyhow::Error> {
	if output.contains("transactionId") {
		Ok(())
	} else {
		context.eprint(
			format!("The registration transaction could not be submitted: {}", output).as_str(),
		);
		Err(anyhow::anyhow!("Registration failed"))
	}
}

#[derive(serde::Deserialize, Debug, Clone)]
struct McEpochConfigJson {
	cardano: MainchainEpochConfig,
}

fn get_current_mainchain_epoch(context: &impl IOContext) -> Result<McEpochNumber, anyhow::Error> {
	let chain_config_json = context
		.read_file(config::CHAIN_CONFIG_FILE_PATH)
		.ok_or_else(|| anyhow::anyhow!("⚠️ The chain configuration file `partner-chains-cli-chain-config.json` is missing or invalid."))?;

	let mc_epoch_config = serde_json::from_str::<McEpochConfigJson>(&chain_config_json)?;
	mc_epoch_config
		.cardano
		.timestamp_to_mainchain_epoch(context.current_timestamp())
		.map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		config::{
			config_fields::POSTGRES_CONNECTION_STRING, CHAIN_CONFIG_FILE_PATH,
			RESOURCES_CONFIG_FILE_PATH,
		},
		pc_contracts_cli_resources::{
			tests::establish_pc_contracts_cli_configuration_io, PcContractsCliResources,
		},
		tests::{MockIO, MockIOContext},
	};
	use serde_json::json;
	use sp_core::offchain::Timestamp;

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_json_file("/path/to/payment.skey", payment_skey_content())
			.with_json_file(CHAIN_CONFIG_FILE_PATH, chain_config_content())
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, resource_config_content())
			.with_expected_io(
				vec![
					intro_msg_io(),
					prompt_mc_payment_key_path_io(),
					vec![establish_pc_contracts_cli_configuration_io(
						None,
						PcContractsCliResources::default(),
					)],
					run_registration_command_io(),
					prompt_for_registration_status_y(),
					show_registration_status_io(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = mock_register3_cmd().run(&mock_context);
		result.expect("should succeed");
	}

	#[test]
	fn registration_call_fails() {
		let mock_context = MockIOContext::new()
			.with_json_file("/path/to/payment.skey", payment_skey_content())
			.with_json_file(CHAIN_CONFIG_FILE_PATH, chain_config_content())
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, resource_config_content())
			.with_expected_io(
				vec![
					intro_msg_io(),
					prompt_mc_payment_key_path_io(),
					vec![establish_pc_contracts_cli_configuration_io(
						None,
						PcContractsCliResources::default(),
					)],
					run_registration_command_fail_io(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = mock_register3_cmd().run(&mock_context);
		result.expect_err("should return error");
	}

	#[test]
	fn not_show_registration_status() {
		let mock_context = MockIOContext::new()
			.with_json_file("/path/to/payment.skey", payment_skey_content())
			.with_json_file(CHAIN_CONFIG_FILE_PATH, chain_config_content())
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, resource_config_content())
			.with_expected_io(
				vec![
					intro_msg_io(),
					prompt_mc_payment_key_path_io(),
					vec![establish_pc_contracts_cli_configuration_io(
						None,
						PcContractsCliResources::default(),
					)],
					run_registration_command_io(),
					prompt_for_registration_status_n(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = mock_register3_cmd().run(&mock_context);
		result.expect("should succeed");
	}

	fn intro_msg_io() -> Vec<MockIO> {
		vec![
            MockIO::print("⚙️ Register as a committee candidate (step 3/3)"),
			MockIO::print("This command will submit the registration message to the mainchain."),
			MockIO::file_read(CHAIN_CONFIG_FILE_PATH), // check if the chain config file exists
			MockIO::print("To proceed with the next command, a payment signing key is required. Please note that this key will not be stored or communicated over the network."),
        ]
	}

	fn prompt_mc_payment_key_path_io() -> Vec<MockIO> {
		vec![MockIO::prompt(
			"Path to mainchain payment signing key file",
			Some("payment.skey"),
			"/path/to/payment.skey",
		)]
	}

	fn prompt_for_registration_status_y() -> Vec<MockIO> {
		vec![MockIO::prompt_yes_no("Show registration status?", true, true)]
	}
	fn prompt_for_registration_status_n() -> Vec<MockIO> {
		vec![MockIO::prompt_yes_no("Show registration status?", true, false)]
	}

	fn show_registration_status_io() -> Vec<MockIO> {
		vec![
        MockIO::print("The registration status will be queried from a db-sync instance for which a valid connection string is required. Please note that this db-sync instance needs to be up and synced with the main chain."),
        MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
        MockIO::current_timestamp(mock_timestamp()),
        MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
        MockIO::prompt("DB-Sync Postgres connection string",POSTGRES_CONNECTION_STRING.default,POSTGRES_CONNECTION_STRING.default.unwrap()),
        MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
        MockIO::file_write_json_contains(RESOURCES_CONFIG_FILE_PATH, &POSTGRES_CONNECTION_STRING.json_pointer(), POSTGRES_CONNECTION_STRING.default.unwrap()),
        MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
        MockIO::set_env_var(
			  "DB_SYNC_POSTGRES_CONNECTION_STRING",  POSTGRES_CONNECTION_STRING.default.unwrap(),
	  	),
        MockIO::set_env_var("CARDANO_SECURITY_PARAMETER", "1234"),
        MockIO::set_env_var("CARDANO_ACTIVE_SLOTS_COEFF", "0.1"),
        MockIO::set_env_var("BLOCK_STABILITY_MARGIN", "0"),
        MockIO::set_env_var("MC__FIRST_EPOCH_TIMESTAMP_MILLIS", "1666742400000"),
        MockIO::set_env_var("MC__FIRST_EPOCH_NUMBER", "1"),
        MockIO::set_env_var("MC__EPOCH_DURATION_MILLIS", "86400000"),
        MockIO::set_env_var("MC__FIRST_SLOT_NUMBER", "4320"),
		MockIO::print("Registrations status for epoch 25:"),
        MockIO::new_tmp_dir(),
        MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
        MockIO::run_command("/path/to/node registration-status --mainchain-pub-key cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7 --mc-epoch-number 25 --chain chain-spec.json --base-path /tmp/MockIOContext_tmp_dir", "{\"epoch\":1,\"validators\":[{\"public_key\":\"cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7\",\"status\":\"Registered\"}]}"),
        MockIO::print("Registration status:"),
        MockIO::print("{\"epoch\":1,\"validators\":[{\"public_key\":\"cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7\",\"status\":\"Registered\"}]}"),
		]
	}

	fn run_registration_command_io() -> Vec<MockIO> {
		vec![
			MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
			MockIO::run_command("./pc-contracts-cli register --network mainnet --sidechain-id 0 --genesis-committee-hash-utxo f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e#0 --threshold-numerator 2 --threshold-denominator 3 --governance-authority 0x00112233445566778899001122334455667788990011223344556677 --atms-kind plain-ecdsa-secp256k1 --registration-utxo cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13#0 --sidechain-public-keys 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1:79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf:1a55db596380bc63f5ee964565359b5ea8e0096c798c3281692df097abbd9aa4b657f887915ad2a52fc85c674ef4044baeaf7149546af93a2744c379b9798f07 --sidechain-signature cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854 --spo-public-key cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7 --spo-signature 448ddd2592a681ee3235aa68356290c3ec93cc1b8b757bf4713a0b6629a3b75028e984a06cd275a99f861f8303dba1778c36feef084ea4a5379775ca13043202 --ada-based-staking --kupo-host localhost --kupo-port 1442  --ogmios-host localhost --ogmios-port 1337  --payment-signing-key-file /path/to/payment.skey", "{\"endpoint\":\"CommitteeCandidateReg\",\"transactionId\":\"1ab93b52d20ce114bfdb48a256ac48f3d8d46d00aec585c38a904b672a70e3a3\"}"),
        ]
	}

	fn run_registration_command_fail_io() -> Vec<MockIO> {
		vec![
			MockIO::file_read(CHAIN_CONFIG_FILE_PATH),
			MockIO::run_command("./pc-contracts-cli register --network mainnet --sidechain-id 0 --genesis-committee-hash-utxo f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e#0 --threshold-numerator 2 --threshold-denominator 3 --governance-authority 0x00112233445566778899001122334455667788990011223344556677 --atms-kind plain-ecdsa-secp256k1 --registration-utxo cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13#0 --sidechain-public-keys 0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1:79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf:1a55db596380bc63f5ee964565359b5ea8e0096c798c3281692df097abbd9aa4b657f887915ad2a52fc85c674ef4044baeaf7149546af93a2744c379b9798f07 --sidechain-signature cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854 --spo-public-key cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7 --spo-signature 448ddd2592a681ee3235aa68356290c3ec93cc1b8b757bf4713a0b6629a3b75028e984a06cd275a99f861f8303dba1778c36feef084ea4a5379775ca13043202 --ada-based-staking --kupo-host localhost --kupo-port 1442  --ogmios-host localhost --ogmios-port 1337  --payment-signing-key-file /path/to/payment.skey", "TxRefNotFound"),
			MockIO::eprint("The registration transaction could not be submitted: TxRefNotFound"),
        ]
	}

	fn mock_register3_cmd() -> Register3Cmd {
		Register3Cmd {
            sidechain_params: config::SidechainParams {
                chain_id: 0,
                threshold_numerator: 2,
                threshold_denominator: 3,
                genesis_committee_utxo: "f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e#0".parse().unwrap(),
                governance_authority: "00112233445566778899001122334455667788990011223344556677".parse().unwrap(),
            },
            registration_utxo: "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13#0".parse().unwrap(),
            sidechain_pub_key: "020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1".parse().unwrap(),
            aura_pub_key: "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf".parse().unwrap(),
            grandpa_pub_key: "1a55db596380bc63f5ee964565359b5ea8e0096c798c3281692df097abbd9aa4b657f887915ad2a52fc85c674ef4044baeaf7149546af93a2744c379b9798f07".parse().unwrap(),
            sidechain_signature: "cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854".to_string(),
			spo_public_key: "cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7".to_string(),
			spo_signature: "448ddd2592a681ee3235aa68356290c3ec93cc1b8b757bf4713a0b6629a3b75028e984a06cd275a99f861f8303dba1778c36feef084ea4a5379775ca13043202".to_string(),
        }
	}

	fn payment_skey_content() -> serde_json::Value {
		serde_json::json!({
			"type": "PaymentSigningKeyShelley_ed25519",
			"description": "Payment Signing Key",
			"cborHex": "5820d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84"
		})
	}

	fn chain_config_content() -> serde_json::Value {
		json!({
			"chain_parameters": chain_parameters_json(),
			"cardano": {
				"security_parameter": 1234,
				"active_slots_coeff": 0.1,
				"first_epoch_timestamp_millis": 1_666_742_400_000_i64,
				"epoch_duration_millis": 86400000,
				"first_epoch_number": 1,
				"first_slot_number": 4320,
				"network": 0
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

	fn chain_parameters_json() -> serde_json::Value {
		json!({
		  "chain_id": 1234,
		  "governance_authority": "0x000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9",
		  "threshold_numerator": 2,
		  "threshold_denominator": 3,
		  "genesis_committee_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0"
		})
	}

	fn resource_config_content() -> serde_json::Value {
		serde_json::json!({
			"substrate_node_base_path": "/path/to/data",
			"substrate_node_executable_path": "/path/to/node",
			"cardano_cli": "cardano-cli",
			"cardano_node_socket_path": "node.socket",
			"cardano_payment_verification_key_file": "payment.vkey",
		})
	}

	fn mock_timestamp() -> Timestamp {
		Timestamp::from_unix_millis(1668658000000u64)
	}
}
