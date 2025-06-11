use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::{apiconfig::*, run_command::*};

fn parse_json_response(result: String) -> Result<JsonValue, String> {
	serde_json::from_str(&result).map_err(|e| e.to_string())
}

pub fn uuid4() -> String {
	uuid::Uuid::new_v4().to_string()
}
pub struct PartnerChainsNode {
	pub config: ApiConfig,
	pub cli: String,
	pub run_command: Runner,
}
// The original code groups pc node subcommands into classes. I've done away with that.
impl PartnerChainsNode {
	pub fn new(config: &ApiConfig) -> Self {
		let cli_config = config.stack_config.tools.partner_chains_node.clone();
		let cli = cli_config.cli.clone();
		let run_command = Runner::new(cli_config.shell.unwrap().clone());

		Self { config: config.clone(), cli, run_command }
	}

	pub fn get_scripts(&self) -> Result<JsonValue, String> {
		let cmd = format!(
			"{} smart-contracts get-scripts
				--genesis-utxo {}
				--ogmios-url {}",
			self.cli,
			self.config.genesis_utxo,
			self.config.stack_config.ogmios_url()
		);
		let response = self.run_command.run(&cmd, self.config.timeouts.main_chain_tx)?;
		parse_json_response(response)
	}

	pub fn reserve_init(&self, payment_key: &str) -> Result<JsonValue, String> {
		let cli = &self.cli;
		let genesis_utxo = &self.config.genesis_utxo;
		let ogmios_url = &self.config.stack_config.ogmios_url();
		let cmd = format!(
			"{cli} smart-contracts reserve init
                --payment-key-file {payment_key}
                --genesis-utxo {genesis_utxo}
                --ogmios-url {ogmios_url}"
		);
		let response = self.run_command.run(&cmd, self.config.timeouts.main_chain_tx)?;
		let parsed_response = parse_json_response(response)?;
		return self.handle_governance_signature(parsed_response);
	}

	pub fn reserve_create(
		&self,
		v_function_hash: &str,
		initial_deposit: i64,
		token: &str,
		payment_key: &str,
	) -> Result<JsonValue, String> {
		let cli = &self.cli;
		let genesis_utxo = &self.config.genesis_utxo;
		let ogmios_url = &self.config.stack_config.ogmios_url();
		let cmd = format!(
			"{cli} smart-contracts reserve create
                --total-accrued-function-script-hash {v_function_hash}
                --initial-deposit-amount {initial_deposit}
                --token {token}
                --payment-key-file {payment_key}
                --genesis-utxo {genesis_utxo}
                --ogmios-url {ogmios_url}"
		);
		let response = self.run_command.run(&cmd, self.config.timeouts.main_chain_tx)?;
		let parsed_response = parse_json_response(response)?;
		return self.handle_governance_signature(parsed_response);
	}

	pub fn reserve_handover(&self, payment_key: &str) -> Result<JsonValue, String> {
		let cli = &self.cli;
		let genesis_utxo = &self.config.genesis_utxo;
		let ogmios_url = &self.config.stack_config.ogmios_url();
		let cmd = format!(
			"{cli} smart-contracts reserve handover
                --payment-key-file {payment_key}
                --genesis-utxo {genesis_utxo}
                --ogmios-url {ogmios_url}"
		);
		let response = self.run_command.run(&cmd, self.config.timeouts.main_chain_tx)?;
		let parsed_response = parse_json_response(response)?;
		return self.handle_governance_signature(parsed_response);
	}

	pub fn reserve_deposit(&self, amount: i64, payment_key: &str) -> Result<JsonValue, String> {
		let cli = &self.cli;
		let genesis_utxo = &self.config.genesis_utxo;
		let ogmios_url = &self.config.stack_config.ogmios_url();
		let cmd = format!(
			"{cli} smart-contracts reserve deposit
                --amount {amount}
                --payment-key-file {payment_key}
                --genesis-utxo {genesis_utxo}
                --ogmios-url {ogmios_url}"
		);
		let response = self.run_command.run(&cmd, self.config.timeouts.main_chain_tx)?;
		let parsed_response = parse_json_response(response)?;
		return self.handle_governance_signature(parsed_response);
	}

	pub fn sign_tx(
		&self,
		transaction_cbor: &str,
		payment_key: &str,
	) -> Result<serde_json::Map<String, JsonValue>, String> {
		let cmd = format!(
			"{} smart-contracts sign-tx
            	--transaction {transaction_cbor}
            	--payment-key-file {payment_key} ",
			self.cli,
		);
		let response = self.run_command.run(&cmd, self.config.timeouts.main_chain_tx)?;
		parse_json_response(response)?
			.as_object()
			.ok_or("expected obj".to_string())
			.cloned()
	}

	pub fn assemble_and_submit_tx(
		&self,
		transaction_cbor: &str,
		witnesses: &[String],
	) -> Result<JsonValue, String> {
		let witnesses_str = witnesses.join(" ");
		let cmd = format!(
			"{} smart-contracts assemble-and-submit-tx
				--transaction {transaction_cbor}
				--witnesses {witnesses_str}
				--ogmios-url {}",
			self.cli,
			self.config.stack_config.ogmios_url()
		);
		let response = self.run_command.run(&cmd, self.config.timeouts.main_chain_tx)?;
		parse_json_response(response)
	}

	fn handle_governance_signature(&self, response: JsonValue) -> Result<JsonValue, String> {
		fn contains_key(data: &JsonValue, key: &str) -> bool {
			match data {
				JsonValue::Object(map) => map.contains_key(key),
				JsonValue::Array(values) => {
					values.iter().any(|v| v.as_object().map_or(false, |o| o.contains_key(key)))
				},
				_ => false,
			}
		}

		if contains_key(&response, "transaction_to_sign") {
			self.handle_multisig(response)
		} else {
			Ok(response)
		}
	}

	fn handle_multisig(&self, response: JsonValue) -> Result<JsonValue, String> {
		fn sign_and_submit_tx(
			pcnode: &PartnerChainsNode,
			tx_cbor: &str,
			config: &ApiConfig,
		) -> Result<JsonValue, String> {
			let mut witnesses = Vec::new();
			for authority in config
				.nodes_config
				.additional_governance_authorities
				.clone()
				.unwrap_or_default()
			{
				let witness = pcnode.sign_tx(tx_cbor, &authority.mainchain_key)?;
				witnesses
					.push(witness["cborHex"].as_str().map(|x| x.to_owned()).ok_or("expected str")?);
			}
			pcnode.assemble_and_submit_tx(tx_cbor, &witnesses)
		}

		let mut result = JsonValue::Null;

		match response {
			JsonValue::Array(arr) => {
				for tx in arr {
					let tx_cbor = tx["transaction_to_sign"]["tx"]["cborHex"]
						.as_str()
						.ok_or("expected str")?;
					result = sign_and_submit_tx(&self, tx_cbor, &self.config)?
				}
			},
			JsonValue::Object(obj) => {
				let tx_cbor =
					obj["transaction_to_sign"]["tx"]["cborHex"].as_str().ok_or("expected str")?;
				result = sign_and_submit_tx(&self, tx_cbor, &self.config)?
			},
			_ => {},
		}
		Ok(result)
	}
}
pub struct CardanoCli {
	pub cli: String,
	pub network: String,
	pub run_command: Runner,
}
impl CardanoCli {
	pub fn new(config: &MainChainConfig, cardano_cli: &Tool) -> Self {
		Self {
			cli: cardano_cli.cli.clone(),
			network: config.network.clone(),
			run_command: Runner::new(cardano_cli.shell.as_ref().unwrap().clone()),
		}
	}
	pub(crate) fn generate_payment_keys(&self) -> Result<(JsonValue, JsonValue), String> {
		log::debug!("Generating payment keys...");
		let cmd = format!(
			"{} latest address key-gen --verification-key-file /dev/stdout --signing-key-file /dev/stdout",
			self.cli
		);
		let result = self.run_command.run(&cmd, 120)?;

		let valid_json_string = format!("[{}]", result.replace("\r", "").replace("}\n{", "},\n{"));
		let skey_vkey_pair =
			serde_json::from_str::<JsonValue>(&valid_json_string).map_err(|e| e.to_string())?;
		let signing_key = skey_vkey_pair[0].clone();
		let verification_key = skey_vkey_pair[1].clone();
		log::debug!("Payment signing key: {signing_key}");
		log::debug!("Payment verification key: {verification_key}");
		Ok((signing_key, verification_key))
	}
	pub fn build_address(&self, payment_vkey: &str) -> Result<String, String> {
		log::debug!("Building address...");
		let cmd = format!(
			"{} latest address build --payment-verification-key {} {}",
			self.cli, payment_vkey, self.network
		);
		self.run_command.run(&cmd, 120).map(|r| r.trim().to_string())
	}
	pub fn get_policy_id(&self, script_file: &str) -> Result<String, String> {
		log::debug!("Calculating policy id...");
		let cmd = format!("{} latest transaction policyid --script-file {}", self.cli, script_file);
		self.run_command.run(&cmd, 120).map(|r| r.trim().to_string())
	}
	pub fn build_tx_with_reference_script(
		&self,
		tx_in: &str,
		address: &str,
		lovelace: i64,
		reference_script_file: &str,
		change_address: &str,
	) -> Result<(String, String), String> {
		log::debug!("Building transaction with reference script...");
		let raw_tx_filepath = format!("/tmp/reference_script_tx_{}.raw", uuid4());
		let cmd = format!(
			"{} latest transaction build
            --tx-in {tx_in}
            --tx-out '{address}+{lovelace}'
            --tx-out-reference-script-file {reference_script_file}
            --change-address {change_address}
            --out-file {raw_tx_filepath}
            {}",
			self.cli, self.network
		);
		Ok((self.run_command.run(&cmd, 120)?, raw_tx_filepath))
	}
	pub fn get_utxos(
		&self,
		address: &str,
	) -> Result<serde_json::Map<std::string::String, JsonValue>, String> {
		let cmd = format!(
			"{} latest query utxo --address {address} {} --out-file /dev/stdout",
			self.cli, self.network
		);
		let result = self.run_command.run(&cmd, 120)?;
		let map: JsonValue = serde_json::from_str(&result).map_err(|e| e.to_string())?;
		Ok(map.as_object().expect("map").clone())
	}

	pub fn sign_transaction(&self, tx_filepath: &str, signing_key: &str) -> Result<String, String> {
		log::debug!("Signing transaction...");
		let signed_tx_filepath = format!("/tmp/signed_tx_{}.signed", uuid4());
		let cmd = format!(
			"{} latest transaction sign
				--tx-body-file {tx_filepath}
				--signing-key-file {signing_key}
				--out-file {signed_tx_filepath}
				{}",
			self.cli, self.network
		);
		let result = self.run_command.run(&cmd, 120)?;
		Ok(signed_tx_filepath)
	}

	pub fn submit_transaction(&self, signed_tx_filepath: &str) -> Result<String, String> {
		log::debug!("Submitting transaction...");
		let cmd = format!(
			"{} latest transaction submit --tx-file {signed_tx_filepath} {}",
			self.cli, self.network
		);
		self.run_command.run(&cmd, 120)
	}

	pub fn get_address_key_hash(
		&self,
		payment_vkey: &str,
	) -> Result<std::string::String, std::string::String> {
		log::debug!("Getting address key hash...");
		let cmd = format!(
			"{} latest address key-hash --payment-verification-key {payment_vkey}",
			self.cli
		);
		self.run_command.run(&cmd, 120)
	}

	pub fn get_token_list_from_address(
		&self,
		address: &str,
	) -> Result<HashMap<String, i64>, String> {
		log::debug!("Getting list of tokens and ADA with amounts...");
		let utxos_json = self.get_utxos(address)?;
		let mut tokens: HashMap<String, i64> = HashMap::new();
		for utxo in utxos_json.keys() {
			for token_policy in utxos_json[utxo]["value"].as_object().expect("obj").keys() {
				if token_policy == "lovelace" {
					*tokens.entry("ADA".to_string()).or_default() +=
						utxos_json[utxo]["value"][token_policy].as_i64().expect("i64");
				} else {
					for token_name in
						utxos_json[utxo]["value"][token_policy].as_object().expect("obj").keys()
					{
						let token = format!("{token_policy}.{token_name}");
						*tokens.entry(token).or_default() += utxos_json[utxo]["value"]
							[token_policy][token_name]
							.as_i64()
							.expect("i64");
					}
				}
			}
		}
		Ok(tokens)
	}
}
pub struct SubstrateApi {
	cardano_cli: CardanoCli,
	partner_chains_node: PartnerChainsNode,
}
impl SubstrateApi {
	pub fn new(config: &ApiConfig) -> Self {
		Self {
			cardano_cli: CardanoCli::new(
				&config.main_chain,
				&config.stack_config.tools.cardano_cli,
			),
			partner_chains_node: PartnerChainsNode::new(config),
		}
	}
}
impl SubstrateApi {
	pub fn cardano_cli(&self) -> &CardanoCli {
		&self.cardano_cli
	}

	pub fn partner_chains_node(&self) -> &PartnerChainsNode {
		&self.partner_chains_node
	}

	pub fn get_mc_balance(&self, address: &str, policy_id: &str) -> Result<i64, String> {
		let tokens_dict = self.cardano_cli().get_token_list_from_address(address)?;
		let balance = tokens_dict.get(policy_id).cloned().unwrap_or_default();
		log::debug!("MC address {address} balance: {balance} {policy_id}");
		Ok(balance)
	}
}
