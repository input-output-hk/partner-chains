use crate::CmdRun;
use crate::cardano_key::get_mc_payment_signing_key_from_file;
use crate::config;
use crate::config::CHAIN_CONFIG_FILE_PATH;
use crate::config::config_fields;
use crate::data_source::set_data_sources_env;
use crate::io::IOContext;
use crate::ogmios::config::establish_ogmios_configuration;
use clap::Parser;
use partner_chains_cardano_offchain::register::Register;
use sidechain_domain::mainchain_epoch::{MainchainEpochConfig, MainchainEpochDerivation};
use sidechain_domain::*;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Register3Cmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	#[arg(long)]
	pub registration_utxo: UtxoId,
	#[arg(long)]
	pub partner_chain_pub_key: SidechainPublicKey,
	#[arg(long)]
	pub aura_pub_key: AuraPublicKey,
	#[arg(long)]
	pub grandpa_pub_key: GrandpaPublicKey,
	#[arg(long)]
	pub partner_chain_signature: SidechainSignature,
	#[arg(long)]
	pub spo_public_key: StakePoolPublicKey,
	#[arg(long)]
	pub spo_signature: MainchainSignature,
}

impl CmdRun for Register3Cmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.print("⚙️ Register as a committee candidate (step 3/3)");
		context.print("This command will submit the registration message to the mainchain.");

		config_fields::GENESIS_UTXO.load_from_file(context).ok_or_else(|| {
			context.eprint(&format!("⚠️ The chain configuration file `{CHAIN_CONFIG_FILE_PATH}` is missing or invalid.\n"));
			context.eprint("⚠️ If you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.\n");
			context.eprint("⚠️ If you are a validator, you can obtain the chain configuration file from the governance authority.\n");
			anyhow::anyhow!("Chain config missing or invalid")
		})?;

		context.print("To proceed with the next command, a payment signing key is required. Please note that this key will not be stored or communicated over the network.");

		let cardano_payment_signing_key_path = config_fields::CARDANO_PAYMENT_SIGNING_KEY_FILE
			.prompt_with_default_from_file_and_save(context);

		let payment_signing_key =
			get_mc_payment_signing_key_from_file(&cardano_payment_signing_key_path, context)?;
		let ogmios_configuration = establish_ogmios_configuration(context)?;
		let candidate_registration = CandidateRegistration {
			stake_ownership: AdaBasedStaking {
				pub_key: self.spo_public_key.clone(),
				signature: self.spo_signature.clone(),
			},
			partner_chain_pub_key: self.partner_chain_pub_key.clone(),
			partner_chain_signature: self.partner_chain_signature.clone(),
			own_pkh: payment_signing_key.to_pub_key_hash(),
			registration_utxo: self.registration_utxo,
			aura_pub_key: self.aura_pub_key.clone(),
			grandpa_pub_key: self.grandpa_pub_key.clone(),
		};
		let offchain = context.offchain_impl(&ogmios_configuration)?;

		let runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		runtime
			.block_on(offchain.register(
				self.common_arguments.retries(),
				self.genesis_utxo,
				&candidate_registration,
				&payment_signing_key,
			))
			.map_err(|e| anyhow::anyhow!("Candidate registration failed: {e:?}!"))?;

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
	set_data_sources_env(context, &chain_config.cardano, &postgres_connection_string);
	Ok(())
}

fn show_registration_status(
	context: &impl IOContext,
	mc_epoch_number: McEpochNumber,
	stake_pool_public_key: StakePoolPublicKey,
) -> Result<(), anyhow::Error> {
	let temp_dir = context.new_tmp_dir();
	let temp_dir_path = temp_dir
		.into_os_string()
		.into_string()
		.expect("PathBuf is a valid UTF-8 String");
	let node_executable = context.current_executable()?;
	let command = format!(
		"{} registration-status --mainchain-pub-key {} --mc-epoch-number {} --chain chain-spec.json --base-path {temp_dir_path}",
		node_executable,
		stake_pool_public_key.to_hex_string(),
		mc_epoch_number
	);
	let output = context.run_command(&command)?;
	context.print("Registration status:");
	context.print(&output);
	Ok(())
}

#[derive(serde::Deserialize, Debug, Clone)]
struct McEpochConfigJson {
	cardano: MainchainEpochConfig,
}

fn get_current_mainchain_epoch(context: &impl IOContext) -> Result<McEpochNumber, anyhow::Error> {
	let chain_config_json = context.read_file(config::CHAIN_CONFIG_FILE_PATH).ok_or_else(|| {
		anyhow::anyhow!(
			"⚠️ The chain configuration file `{CHAIN_CONFIG_FILE_PATH}` is missing or invalid."
		)
	})?;

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
			CHAIN_CONFIG_FILE_PATH, RESOURCES_CONFIG_FILE_PATH,
			config_fields::POSTGRES_CONNECTION_STRING,
		},
		ogmios::config::tests::{
			default_ogmios_config_json, default_ogmios_service_config,
			establish_ogmios_configuration_io,
		},
		tests::{MockIO, MockIOContext, OffchainMock, OffchainMocks},
		verify_json,
	};
	use hex_literal::hex;
	use partner_chains_cardano_offchain::OffchainError;
	use serde_json::json;
	use sp_core::offchain::Timestamp;

	#[test]
	fn happy_path() {
		let offchain_mock = OffchainMock::new().with_register(
			genesis_utxo(),
			new_candidate_registration(),
			payment_signing_key(),
			Ok(Some(McTxHash::default())),
		);

		let mock_context = MockIOContext::new()
			.with_json_file("/path/to/payment.skey", payment_skey_content())
			.with_json_file(CHAIN_CONFIG_FILE_PATH, chain_config_content())
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, resource_config_content())
			.with_offchain_mocks(OffchainMocks::new_with_mock(
				"http://localhost:1337",
				offchain_mock,
			))
			.with_expected_io(
				vec![
					intro_msg_io(),
					prompt_mc_payment_key_path_io(),
					get_ogmios_config(),
					prompt_for_registration_status_y(),
					show_registration_status_io(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = mock_register3_cmd().run(&mock_context);
		result.expect("should succeed");
		verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, final_resources_config_json());
	}

	#[test]
	fn registration_call_fails() {
		let offchain_mock = OffchainMock::new().with_register(
			genesis_utxo(),
			new_candidate_registration(),
			payment_signing_key(),
			Err(OffchainError::InternalError("test error".to_string())),
		);
		let mock_context = MockIOContext::new()
			.with_json_file("/path/to/payment.skey", payment_skey_content())
			.with_json_file(CHAIN_CONFIG_FILE_PATH, chain_config_content())
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, resource_config_content())
			.with_offchain_mocks(OffchainMocks::new_with_mock(
				"http://localhost:1337",
				offchain_mock,
			))
			.with_expected_io(
				vec![intro_msg_io(), prompt_mc_payment_key_path_io(), get_ogmios_config()]
					.into_iter()
					.flatten()
					.collect::<Vec<MockIO>>(),
			);

		let result = mock_register3_cmd().run(&mock_context);
		result.expect_err("should return error");
	}

	#[test]
	fn not_show_registration_status() {
		let offchain_mock = OffchainMock::new().with_register(
			genesis_utxo(),
			new_candidate_registration(),
			payment_signing_key(),
			Ok(Some(McTxHash::default())),
		);
		let mock_context = MockIOContext::new()
			.with_json_file("/path/to/payment.skey", payment_skey_content())
			.with_json_file(CHAIN_CONFIG_FILE_PATH, chain_config_content())
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, resource_config_content())
			.with_offchain_mocks(OffchainMocks::new_with_mock(
				"http://localhost:1337",
				offchain_mock,
			))
			.with_expected_io(
				vec![
					intro_msg_io(),
					prompt_mc_payment_key_path_io(),
					get_ogmios_config(),
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
			MockIO::print(
				"To proceed with the next command, a payment signing key is required. Please note that this key will not be stored or communicated over the network.",
			),
		]
	}

	fn prompt_mc_payment_key_path_io() -> Vec<MockIO> {
		vec![MockIO::prompt(
			"path to the payment signing key file",
			Some("payment.skey"),
			"/path/to/payment.skey",
		)]
	}

	fn get_ogmios_config() -> Vec<MockIO> {
		vec![establish_ogmios_configuration_io(
			Some(default_ogmios_service_config()),
			default_ogmios_service_config(),
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
			MockIO::print(
				"The registration status will be queried from a db-sync instance for which a valid connection string is required. Please note that this db-sync instance needs to be up and synced with the main chain.",
			),
			MockIO::current_timestamp(mock_timestamp()),
			MockIO::prompt(
				"DB-Sync Postgres connection string",
				POSTGRES_CONNECTION_STRING.default,
				POSTGRES_CONNECTION_STRING.default.unwrap(),
			),
			MockIO::set_env_var(
				"DB_SYNC_POSTGRES_CONNECTION_STRING",
				POSTGRES_CONNECTION_STRING.default.unwrap(),
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
			MockIO::run_command(
				"<mock executable> registration-status --mainchain-pub-key 0xcef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7 --mc-epoch-number 25 --chain chain-spec.json --base-path /tmp/MockIOContext_tmp_dir",
				"{\"epoch\":1,\"validators\":[{\"public_key\":\"cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7\",\"status\":\"Registered\"}]}",
			),
			MockIO::print("Registration status:"),
			MockIO::print(
				"{\"epoch\":1,\"validators\":[{\"public_key\":\"cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7\",\"status\":\"Registered\"}]}",
			),
		]
	}

	fn mock_register3_cmd() -> Register3Cmd {
		Register3Cmd {
			common_arguments: crate::CommonArguments { retry_delay_seconds: 5, retry_count: 59 },
            genesis_utxo: genesis_utxo(),
            registration_utxo: "cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13#0".parse().unwrap(),
            partner_chain_pub_key: "020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1".parse().unwrap(),
            aura_pub_key: "79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf".parse().unwrap(),
            grandpa_pub_key: "1a55db596380bc63f5ee964565359b5ea8e0096c798c3281692df097abbd9aa4b657f887915ad2a52fc85c674ef4044baeaf7149546af93a2744c379b9798f07".parse().unwrap(),
            partner_chain_signature: SidechainSignature(hex_literal::hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854").to_vec()),
			spo_public_key: StakePoolPublicKey(hex_literal::hex!("cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7")),
			spo_signature: MainchainSignature(hex_literal::hex!("aaa39fbf163ed77c69820536f5dc22854e7e13f964f1e077efde0844a09bde64c1aab4d2b401e0fe39b43c91aa931cad26fa55c8766378462c06d86c85134801")),
        }
	}

	fn genesis_utxo() -> UtxoId {
		"f17e6d3aa72095e04489d13d776bf05a66b5a8c49d89397c28b18a1784b9950e#0"
			.parse()
			.unwrap()
	}

	fn payment_skey_content() -> serde_json::Value {
		serde_json::json!({
			"type": "PaymentSigningKeyShelley_ed25519",
			"description": "Payment Signing Key",
			"cborHex": "5820d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84"
		})
	}

	fn payment_signing_key() -> Vec<u8> {
		hex!("d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84").to_vec()
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
				"slot_duration_millis": 1000,
				"network": "mainnet"
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
				  "partner_chain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"
				},
				{
				  "aura_pub_key": "0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
				  "grandpa_pub_key": "0xd17c2d7823ebf260fd138f2d7e27d114c0145d968b5ff5006125f2414fadae69",
				  "partner_chain_pub_key": "0x0390084fdbf27d2b79d26a4f13f0ccd982cb755a661969143c37cbc49ef5b91f27"
				}
			],
		})
	}

	fn final_resources_config_json() -> serde_json::Value {
		json!({
			"cardano_payment_signing_key_file": "/path/to/payment.skey",
			"cardano_payment_verification_key_file": "payment.vkey",
			"db_sync_postgres_connection_string": "postgresql://postgres-user:postgres-password@localhost:5432/cexplorer",
			"ogmios": default_ogmios_config_json(),
			"substrate_node_base_path": "/path/to/data",
			"substrate_node_executable_path": "/path/to/node"
		})
	}

	fn chain_parameters_json() -> serde_json::Value {
		json!({
		  "genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000000#0"
		})
	}

	fn resource_config_content() -> serde_json::Value {
		serde_json::json!({
			"substrate_node_base_path": "/path/to/data",
			"substrate_node_executable_path": "/path/to/node",
			"cardano_payment_verification_key_file": "payment.vkey",
		})
	}

	fn mock_timestamp() -> Timestamp {
		Timestamp::from_unix_millis(1668658000000u64)
	}

	fn new_candidate_registration() -> CandidateRegistration {
		CandidateRegistration {
			stake_ownership: AdaBasedStaking {
				pub_key: StakePoolPublicKey(hex!("cef2d1630c034d3b9034eb7903d61f419a3074a1ad01d4550cc72f2b733de6e7")),
				signature: MainchainSignature(hex!("aaa39fbf163ed77c69820536f5dc22854e7e13f964f1e077efde0844a09bde64c1aab4d2b401e0fe39b43c91aa931cad26fa55c8766378462c06d86c85134801"))
			},
			partner_chain_pub_key: SidechainPublicKey(hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec()),
			partner_chain_signature: SidechainSignature(hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854").to_vec()),
			own_pkh: MainchainKeyHash(hex!("7fa48bb8fb5d6804fad26237738ce490d849e4567161e38ab8415ff3")),
			registration_utxo: UtxoId { tx_hash: McTxHash(hex!("cdefe62b0a0016c2ccf8124d7dda71f6865283667850cc7b471f761d2bc1eb13")), index: UtxoIndex(0) },
			aura_pub_key: AuraPublicKey(hex!("79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf").to_vec()),
			grandpa_pub_key: GrandpaPublicKey(hex!("1a55db596380bc63f5ee964565359b5ea8e0096c798c3281692df097abbd9aa4b657f887915ad2a52fc85c674ef4044baeaf7149546af93a2744c379b9798f07").to_vec())
		}
	}
}
