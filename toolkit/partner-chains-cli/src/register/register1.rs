use crate::config::config_values::DEFAULT_CHAIN_NAME;
use crate::config::KEYS_FILE_PATH;
use crate::generate_keys::keystore_path;
use crate::io::IOContext;
use crate::keystore::CROSS_CHAIN;
use crate::{config::config_fields, *};
use anyhow::anyhow;
use cli_commands::registration_signatures::RegisterValidatorMessage;
use cli_commands::signing::sc_public_key_and_signature_for_datum;
use partner_chains_cardano_offchain::csl::NetworkTypeExt;
use select_utxo::{filter_utxos, query_utxos, select_from_utxos, ValidUtxo};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sidechain_domain::{NetworkType, SidechainPublicKey, UtxoId};
use sp_core::bytes::from_hex;
use sp_core::{ecdsa, Pair};

#[derive(Debug, clap::Parser)]
pub struct Register1Cmd {}

impl CmdRun for Register1Cmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.print("⚙️ Registering as a committee candidate (step 1/3)");
		let genesis_utxo = load_chain_config_field(context, &config_fields::GENESIS_UTXO)?;
		let cardano_network = config_fields::CARDANO_NETWORK.load_from_file(context).ok_or_else(|| {
            context.eprint("⚠️ Cardano network is not specified in the chain configuration file `partner-chains-cli-chain-config.json`");
            anyhow!("failed to read cardano network")
        })?;

		let node_data_base_path = config_fields::SUBSTRATE_NODE_DATA_BASE_PATH
			.load_from_file(context)
			.ok_or(anyhow::anyhow!(
				"⚠️ Keystore not found. Please run the `generate-keys` command first"
			))?;

		let GeneratedKeysFileContent { sidechain_pub_key, aura_pub_key, grandpa_pub_key } =
			read_generated_keys(context).map_err(|e| {
			    context.eprint("⚠️ The keys file `partner-chains-cli-keys.json` is missing or invalid. Please run the `generate-keys` command first");
				anyhow!(e)
			})?;

		context.print("This wizard will query your UTXOs using address derived from the payment verification key and Ogmios service");
		let address = derive_address(context, cardano_network)?;
		let ogmios_configuration =
			pc_contracts_cli_resources::prompt_ogmios_configuration(context)?;
		let utxo_query_result = query_utxos(context, &ogmios_configuration, &address)?;

		let valid_utxos: Vec<ValidUtxo> = filter_utxos(utxo_query_result);

		if valid_utxos.is_empty() {
			context.eprint("⚠️ No UTXOs found for the given address");
			context.eprint(
				"The registering transaction requires at least one UTXO to be present at the address.",
			);
			return Err(anyhow::anyhow!("No UTXOs found"));
		};

		let input_utxo: UtxoId =
			select_from_utxos(context, "Select UTXO to use for registration", valid_utxos)?;

		context.print("Please do not spend this UTXO, it needs to be consumed by the registration transaction.");
		context.print("");

		let sidechain_pub_key_typed: SidechainPublicKey =
			SidechainPublicKey(from_hex(&sidechain_pub_key).map_err(|e| {
				context.eprint(&format!("⚠️ Failed to decode sidechain public key: {e}"));
				anyhow!(e)
			})?);

		let registration_message = RegisterValidatorMessage {
			genesis_utxo,
			sidechain_pub_key: sidechain_pub_key_typed,
			input_utxo,
		};

		let ecdsa_pair = get_ecdsa_pair_from_file(
			context,
			&keystore_path(&node_data_base_path, DEFAULT_CHAIN_NAME),
			&sidechain_pub_key,
		)
		.map_err(|e| {
			context.eprint(&format!("⚠️ Failed to read sidechain key from the keystore: {e}"));
			anyhow!(e)
		})?;

		let sidechain_signature =
			sign_registration_message_with_sidechain_key(registration_message, ecdsa_pair)?;

		context.print("Run the following command to generate signatures on the next step. It has to be executed on the machine with your SPO cold signing key.");
		context.print("");
		context.print(&format!("./partner-chains-cli register2 \\\n --genesis-utxo {genesis_utxo} \\\n --registration-utxo {input_utxo} \\\n --aura-pub-key {aura_pub_key} \\\n --grandpa-pub-key {grandpa_pub_key} \\\n --sidechain-pub-key {sidechain_pub_key} \\\n --sidechain-signature {sidechain_signature}"));

		Ok(())
	}
}

fn get_ecdsa_pair_from_file<C: IOContext>(
	context: &C,
	keystore_path: &str,
	sidechain_pub_key: &str,
) -> Result<ecdsa::Pair, anyhow::Error> {
	let mut seed_phrase_file_name = CROSS_CHAIN.key_type_hex();
	seed_phrase_file_name.push_str(sidechain_pub_key.replace("0x", "").as_str());
	let seed_phrase_file_path = format!("{keystore_path}/{seed_phrase_file_name}");
	let seed = context
		.read_file(&seed_phrase_file_path)
		.ok_or_else(|| anyhow::anyhow!("seed phrase file not found"))?;
	let stripped_quotes = seed.trim_matches('\"');
	Ok(ecdsa::Pair::from_string(stripped_quotes, None)?)
}

fn sign_registration_message_with_sidechain_key(
	message: RegisterValidatorMessage,
	ecdsa_pair: ecdsa::Pair,
) -> Result<String, anyhow::Error> {
	let seed = ecdsa_pair.seed();
	let secret_key = secp256k1::SecretKey::from_slice(&seed).map_err(|e| anyhow!(e))?;
	let (_, sig) = sc_public_key_and_signature_for_datum(secret_key, message.clone());
	Ok(hex::encode(sig.serialize_compact()))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneratedKeysFileContent {
	pub sidechain_pub_key: String,
	pub aura_pub_key: String,
	pub grandpa_pub_key: String,
}

pub fn read_generated_keys<C: IOContext>(context: &C) -> anyhow::Result<GeneratedKeysFileContent> {
	let keys_file_content = context
		.read_file(KEYS_FILE_PATH)
		.ok_or_else(|| anyhow::anyhow!("failed to read keys file"))?;
	Ok(serde_json::from_str(&keys_file_content)?)
}

pub fn load_chain_config_field<C: IOContext, T>(
	context: &C,
	field: &config::ConfigFieldDefinition<T>,
) -> Result<T, anyhow::Error>
where
	T: DeserializeOwned,
{
	field.load_from_file(context).ok_or_else(|| {
		context.eprint("⚠️ The chain configuration file `partner-chains-cli-chain-config.json` is missing or invalid.\n If you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.\n If you are a validator, you can obtain the chain configuration file from the governance authority.");
		anyhow::anyhow!("failed to read {}", field.path.join("."))
	})
}

fn derive_address<C: IOContext>(
	context: &C,
	cardano_network: NetworkType,
) -> Result<String, anyhow::Error> {
	let cardano_payment_verification_key_file =
		config_fields::CARDANO_PAYMENT_VERIFICATION_KEY_FILE
			.prompt_with_default_from_file_and_save(context);
	let key_bytes: [u8; 32] =
		cardano_key::get_key_bytes_from_file(&cardano_payment_verification_key_file, context)?;
	let address =
		partner_chains_cardano_offchain::csl::payment_address(&key_bytes, cardano_network.to_csl());
	address.to_bech32(None).map_err(|e| anyhow!(e.to_string()))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::tests::{MockIO, MockIOContext};
	use ogmios::OgmiosRequest;
	use pc_contracts_cli_resources::default_ogmios_service_config;
	use pc_contracts_cli_resources::tests::prompt_ogmios_configuration_io;
	use select_utxo::tests::{mock_5_valid_utxos_rows, mock_result_5_valid};

	const PAYMENT_VKEY_PATH: &str = "payment.vkey";

	#[test]
	fn happy_path() {
		let resource_config_without_cardano_fields = serde_json::json!({
			"substrate_node_base_path": "/path/to/data",
		});

		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_content())
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_without_cardano_fields)
			.with_json_file(KEYS_FILE_PATH, generated_keys_file_content())
			.with_file(ECDSA_KEY_PATH, ECDSA_KEY_FILE_CONTENT)
			.with_file(PAYMENT_VKEY_PATH, PAYMENT_VKEY_CONTENT)
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					read_resource_config_io(),
					derive_address_io(),
					query_utxos_io(),
					select_utxo_io(),
					sign_registration_message_io(),
					output_io(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		result.expect("should succeed");
	}

	#[test]
	fn report_error_if_chain_config_file_is_missing() {
		let mock_context = MockIOContext::new().with_expected_io(
			vec![intro_msg_io(), invalid_chain_config_io()]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
		);

		let result = Register1Cmd {}.run(&mock_context);
		result.expect_err("should return error");
	}

	#[test]
	fn report_error_if_chain_config_fields_are_missing() {
		let mock_context = MockIOContext::new()
			.with_json_file("partner-chains-cli-chain-config.json", serde_json::json!({}))
			.with_expected_io(
				vec![
					intro_msg_io(),
					vec![MockIO::file_read("partner-chains-cli-chain-config.json")],
					invalid_chain_config_io(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		result.expect_err("should return error");
	}

	#[test]
	fn saved_prompt_fields_are_loaded_without_prompting() {
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_content())
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_content())
			.with_json_file(KEYS_FILE_PATH, generated_keys_file_content())
			.with_file(PAYMENT_VKEY_PATH, PAYMENT_VKEY_CONTENT)
			.with_file(ECDSA_KEY_PATH, ECDSA_KEY_FILE_CONTENT)
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					read_resource_config_io(),
					derive_address_io(),
					query_utxos_io(),
					select_utxo_io(),
					sign_registration_message_io(),
					output_io(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		assert!(result.is_ok());
	}

	#[test]
	fn fail_if_cardano_network_is_not_specified() {
		let chain_config_without_cardano_network: serde_json::Value = serde_json::json!({
			"chain_parameters": {
				"genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000001#0",
			},
		});

		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_without_cardano_network)
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_content())
			.with_json_file(KEYS_FILE_PATH, generated_keys_file_content())
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					vec![
						MockIO::eprint("⚠️ Cardano network is not specified in the chain configuration file `partner-chains-cli-chain-config.json`"),
					]
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		assert!(result.is_err());
	}

	#[test]
	fn report_error_if_payment_file_is_invalid() {
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_content())
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_content())
			.with_json_file(KEYS_FILE_PATH, generated_keys_file_content())
			.with_file(PAYMENT_VKEY_PATH, "invalid content")
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					read_resource_config_io(),
					derive_address_io(),
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		assert!(result.is_err());
		assert!(result
			.unwrap_err()
			.to_string()
			.contains("Failed to parse Cardano key file payment.vkey"));
	}

	#[test]
	fn utxo_query_error() {
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_content())
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_content())
			.with_json_file(KEYS_FILE_PATH, generated_keys_file_content())
			.with_file(PAYMENT_VKEY_PATH, PAYMENT_VKEY_CONTENT)
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					read_resource_config_io(),
					vec![
    					address_and_utxo_msg_io(),
    					prompt_cardano_payment_verification_key_file_io(),
    					read_payment_verification_key_file_io(),
    					prompt_ogmios_configuration_io(&default_ogmios_service_config(), &default_ogmios_service_config()),
    					MockIO::print("⚙️ Querying UTXOs of addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy from Ogmios at http://localhost:1337..."),
    					MockIO::ogmios_request(
    						"http://localhost:1337",
    						OgmiosRequest::QueryUtxo {
    							address: "addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy"
    								.into(),
    						},
    						Err(anyhow!("Ogmios request failed!")),
    					),
					]
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		assert!(result.is_err());
		assert_eq!(result.unwrap_err().to_string(), "Ogmios request failed!".to_owned());
	}

	#[test]
	fn should_error_with_missing_public_keys_file() {
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_content())
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_content())
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					read_resource_config_io(),
					vec![MockIO::eprint("⚠️ The keys file `partner-chains-cli-keys.json` is missing or invalid. Please run the `generate-keys` command first")],
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		assert!(result.is_err());
	}

	#[test]
	fn should_error_with_missing_private_keys_in_storage() {
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_content())
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_content())
			.with_file(PAYMENT_VKEY_PATH, PAYMENT_VKEY_CONTENT)
			.with_json_file(KEYS_FILE_PATH, generated_keys_file_content())
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					read_resource_config_io(),
					derive_address_io(),
					query_utxos_io(),
					select_utxo_io(),
					vec![
						MockIO::file_read(ECDSA_KEY_PATH),
						MockIO::eprint("⚠️ Failed to read sidechain key from the keystore: seed phrase file not found"),
					],
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		assert!(result.is_err());
	}

	#[test]
	fn should_error_on_invalid_seed_phrase() {
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_PATH, chain_config_content())
			.with_json_file(RESOURCE_CONFIG_PATH, resource_config_content())
			.with_json_file(KEYS_FILE_PATH, generated_keys_file_content())
			.with_file(PAYMENT_VKEY_PATH, PAYMENT_VKEY_CONTENT)
			.with_file(ECDSA_KEY_PATH, "invalid seed phrase")
			.with_expected_io(
				vec![
					intro_msg_io(),
					read_chain_config_io(),
					read_resource_config_io(),
					derive_address_io(),
					query_utxos_io(),
					select_utxo_io(),
					vec![
						MockIO::file_read(ECDSA_KEY_PATH),
						MockIO::eprint(
							"⚠️ Failed to read sidechain key from the keystore: Invalid phrase",
						),
					],
				]
				.into_iter()
				.flatten()
				.collect::<Vec<MockIO>>(),
			);

		let result = Register1Cmd {}.run(&mock_context);
		assert!(result.is_err());
	}

	const CHAIN_CONFIG_PATH: &str = "partner-chains-cli-chain-config.json";
	const RESOURCE_CONFIG_PATH: &str = "partner-chains-cli-resources-config.json";

	fn chain_config_content() -> serde_json::Value {
		serde_json::json!({
			"chain_parameters": {
				"genesis_utxo": "0000000000000000000000000000000000000000000000000000000000000001#0",
			},
			"cardano": {
				"network": "testnet"
			}
		})
	}

	fn generated_keys_file_content() -> serde_json::Value {
		serde_json::json!({
		  "sidechain_pub_key": "0x031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6",
		  "aura_pub_key": "0xdf883ee0648f33b6103017b61be702017742d501b8fe73b1d69ca0157460b777",
		  "grandpa_pub_key": "0x5a091a06abd64f245db11d2987b03218c6bd83d64c262fe10e3a2a1230e90327"
		})
	}

	const PAYMENT_VKEY_CONTENT: &str = r#"
{
    "type": "StakePoolVerificationKey_ed25519",
    "description": "Stake Pool Operator Verification Key",
    "cborHex": "5820a35ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be7e3c5e88"
}
"#;

	const ECDSA_KEY_FILE_CONTENT: &str =
		"\"end fury stamp spatial focus tired video tumble good critic tail hood\"";

	fn resource_config_content() -> serde_json::Value {
		serde_json::json!({
			"substrate_node_base_path": "/path/to/data",
			"cardano_payment_verification_key_file": "payment.vkey"
		})
	}

	fn intro_msg_io() -> Vec<MockIO> {
		vec![MockIO::print("⚙️ Registering as a committee candidate (step 1/3)")]
	}
	fn read_chain_config_io() -> Vec<MockIO> {
		vec![
			MockIO::file_read(CHAIN_CONFIG_PATH), // genesis utxo
			MockIO::file_read(CHAIN_CONFIG_PATH), // cardano network
		]
	}

	fn read_resource_config_io() -> Vec<MockIO> {
		vec![
			MockIO::file_read(RESOURCE_CONFIG_PATH), // substrate node base path
			MockIO::file_read(KEYS_FILE_PATH),       // generated keys file
		]
	}

	fn address_and_utxo_msg_io() -> MockIO {
		MockIO::Group(vec![
			MockIO::print("This wizard will query your UTXOs using address derived from the payment verification key and Ogmios service"),
		])
	}

	fn prompt_cardano_payment_verification_key_file_io() -> MockIO {
		MockIO::Group(vec![
			MockIO::file_read(RESOURCE_CONFIG_PATH),
			MockIO::prompt(
				"path to the payment verification file",
				Some(PAYMENT_VKEY_PATH),
				PAYMENT_VKEY_PATH,
			),
			MockIO::file_read(RESOURCE_CONFIG_PATH),
			MockIO::file_write_json(
				RESOURCE_CONFIG_PATH,
				serde_json::json!({"substrate_node_base_path": "/path/to/data", "cardano_payment_verification_key_file": PAYMENT_VKEY_PATH}),
			),
		])
	}

	fn read_payment_verification_key_file_io() -> MockIO {
		MockIO::file_read("payment.vkey")
	}

	fn derive_address_io() -> Vec<MockIO> {
		vec![
			address_and_utxo_msg_io(),
			prompt_cardano_payment_verification_key_file_io(),
			read_payment_verification_key_file_io(),
		]
	}

	fn query_utxos_io() -> Vec<MockIO> {
		vec![
			prompt_ogmios_configuration_io(
				&default_ogmios_service_config(),
				&default_ogmios_service_config(),
			),
			crate::select_utxo::tests::query_utxos_io(
				"addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy",
				"http://localhost:1337",
				mock_result_5_valid(),
			),
		]
	}

	fn select_utxo_io() -> Vec<MockIO> {
		vec![
		MockIO::prompt_multi_option("Select UTXO to use for registration", mock_5_valid_utxos_rows(), "4704a903b01514645067d851382efd4a6ed5d2ff07cf30a538acc78fed7c4c02#93 (1100000 lovelace)"),

		MockIO::print("Please do not spend this UTXO, it needs to be consumed by the registration transaction."),
		MockIO::print(""),
		]
	}

	fn output_io() -> Vec<MockIO> {
		vec![
		MockIO::print("Run the following command to generate signatures on the next step. It has to be executed on the machine with your SPO cold signing key."),
		MockIO::print(""),
		MockIO::print("./partner-chains-cli register2 \\\n --genesis-utxo 0000000000000000000000000000000000000000000000000000000000000001#0 \\\n --registration-utxo 4704a903b01514645067d851382efd4a6ed5d2ff07cf30a538acc78fed7c4c02#93 \\\n --aura-pub-key 0xdf883ee0648f33b6103017b61be702017742d501b8fe73b1d69ca0157460b777 \\\n --grandpa-pub-key 0x5a091a06abd64f245db11d2987b03218c6bd83d64c262fe10e3a2a1230e90327 \\\n --sidechain-pub-key 0x031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6 \\\n --sidechain-signature 6e295e36a6b11d8b1c5ec01ac8a639b466fbfbdda94b39ea82b0992e303d58543341345fc705e09c7838786ba0bc746d9038036f66a36d1127d924c4a0228bec")
		]
	}

	const ECDSA_KEY_PATH: &str = "/path/to/data/chains/partner_chains_template/keystore/63726368031e75acbf45ef8df98bbe24b19b28fff807be32bf88838c30c0564d7bec5301f6";

	fn sign_registration_message_io() -> Vec<MockIO> {
		vec![MockIO::file_read(ECDSA_KEY_PATH)]
	}

	fn invalid_chain_config_io() -> Vec<MockIO> {
		vec![MockIO::eprint("⚠️ The chain configuration file `partner-chains-cli-chain-config.json` is missing or invalid.\n If you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.\n If you are a validator, you can obtain the chain configuration file from the governance authority.")]
	}
}
