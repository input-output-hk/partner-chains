use crate::CmdRun;
use crate::cmd_traits::{GetScriptsData, InitGovernance};
use crate::config::config_fields::{BOOTNODES, SUBSTRATE_NODE_DATA_BASE_PATH};
use crate::generate_keys::network_key_path;
use crate::io::IOContext;
use crate::prepare_configuration::prepare_main_chain_config::prepare_main_chain_config;
use crate::prepare_configuration::select_genesis_utxo::select_genesis_utxo;
use anyhow::Context;
use libp2p_identity::Keypair;
use ogmios_client::query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId};
use ogmios_client::query_network::QueryNetwork;
use ogmios_client::transactions::Transactions;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::cardano_keys::CardanoPaymentSigningKey;
use partner_chains_cardano_offchain::csl::NetworkTypeExt;
use partner_chains_cardano_offchain::governance::MultiSigParameters;
use partner_chains_cardano_offchain::scripts_data::{ScriptsData, get_scripts_data};
use sidechain_domain::{McTxHash, UtxoId};
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::vec;

mod init_governance;
mod prepare_cardano_params;
mod prepare_main_chain_config;
mod select_genesis_utxo;

#[derive(Clone, Debug, clap::Parser)]
pub struct PrepareConfigurationCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
}

impl CmdRun for PrepareConfigurationCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		context.eprint(INTRO);
		establish_bootnodes(context)?;
		let (genesis_utxo, private_key, ogmios_config) = select_genesis_utxo(context)?;
		if let Some(_tx_id) = init_governance::run_init_governance(
			self.common_arguments.retries(),
			genesis_utxo,
			&private_key,
			&ogmios_config,
			context,
		)? {
			prepare_main_chain_config(context, &ogmios_config, genesis_utxo)?;
			context.eprint("🚀 Chain configuration wizards completed successufully!");
			Ok(())
		} else {
			context
				.eprint("Chain governance has not been initialized. Please run the wizard again.");
			Ok(())
		}
	}
}

fn establish_bootnodes(context: &impl IOContext) -> anyhow::Result<()> {
	match peer_id_from_config(context)? {
		Some(peer_id) => {
			let configure = context.prompt_yes_no(
				&format!(
					"Do you want to configure a single bootnode with your peer id '{peer_id}'?"
				),
				true,
			);
			if configure {
				configure_bootnode(peer_id, context)
			} else {
				Ok(BOOTNODES.save_to_file(&vec![], context))
			}
		},
		None => {
			let continue_without_bootnode =
				context.prompt_yes_no("Could not read network secret key from the base directory. Do you want to continue without configuring bootnode?", true);
			if continue_without_bootnode {
				Ok(BOOTNODES.save_to_file(&vec![], context))
			} else {
				std::process::exit(0)
			}
		},
	}
}

fn configure_bootnode(peer_id: String, context: &impl IOContext) -> anyhow::Result<()> {
	let (default_protocol, default_hostname_or_ip, default_port) = read_bootnode_defaults(context);

	let protocol = choose_protocol(context, default_protocol);

	let port: u16 = context
		.prompt(CHOOSE_PORT_PROMPT, Some(&default_port.to_string()))
		.parse()
		.context("⚠️ Port must be a valid number")?;

	let default_address = if default_protocol == protocol {
		&default_hostname_or_ip
	} else {
		protocol.default_address()
	};

	let bootnode = match protocol {
		Protocol::Dns => {
			let hostname = context.prompt(CHOOSE_HOSTNAME_PROMPT, Some(default_address));
			dns_bootnode(&hostname, port, &peer_id)
		},
		Protocol::Ipv4 => {
			let ipv4_address = context.prompt(CHOOSE_IP_ADDRESS_PROMPT, Some(default_address));
			let _: Ipv4Addr = ipv4_address.parse().context("⚠️ Invalid IP address")?;

			ipv4_bootnode(&ipv4_address, port, &peer_id)
		},
	};

	BOOTNODES.save_to_file(&vec![bootnode], context);
	context.eprint(&outro(context.chain_config_file_path()));
	Ok(())
}

fn read_bootnode_defaults(context: &impl IOContext) -> (Protocol, String, u16) {
	let (default_protocol, default_hostname_or_ip, default_port) = deconstruct_bootnode(
		BOOTNODES
			.load_from_file(context)
			.and_then(|bootnodes| bootnodes.into_iter().next()),
	)
	.unwrap_or((Protocol::Dns, Protocol::Dns.default_address().to_string(), DEFAULT_PORT));
	(default_protocol, default_hostname_or_ip, default_port)
}

fn choose_protocol(context: &impl IOContext, default_protocol: Protocol) -> Protocol {
	let mut protocols: Vec<String> = vec![Protocol::Dns.into(), Protocol::Ipv4.into()];
	// default protocol should be the first in the list
	protocols
		.sort_by_key(|protocol| if *protocol != String::from(default_protocol) { 1 } else { 0 });

	Protocol::from_str(&context.prompt_multi_option(CHOOSE_PROTOCOL_PROMPT, protocols))
		.expect("Invalid protocol cannot be chosen from valid options only")
}

fn deconstruct_bootnode(bootnode_opt: Option<String>) -> Option<(Protocol, String, u16)> {
	let bootnode = bootnode_opt?;
	let mut parts = bootnode.split('/').skip(1);
	let protocol = parts.next()?.parse().ok()?;
	let hostname = parts.next()?.to_string();
	parts.next()?;
	let port = parts.next()?.parse().ok()?;
	Some((protocol, hostname, port))
}

fn peer_id_from_config(context: &impl IOContext) -> anyhow::Result<Option<String>> {
	let substrate_node_base_path =
		SUBSTRATE_NODE_DATA_BASE_PATH.prompt_with_default_from_file_and_save(context);

	let network_key_path = network_key_path(&substrate_node_base_path);
	Ok(match context.read_file(&network_key_path).as_deref() {
		Some(network_key) => Some(peer_id_from_network_key(network_key)?),
		None => {
			context.print(&format!("Could not read network key from {}", network_key_path));
			None
		},
	})
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Protocol {
	Dns,
	Ipv4,
}

impl Protocol {
	fn default_address(&self) -> &str {
		match self {
			Protocol::Dns => "localhost",
			Protocol::Ipv4 => "127.0.0.1",
		}
	}
}

impl From<Protocol> for String {
	fn from(value: Protocol) -> Self {
		match value {
			Protocol::Dns => "hostname".to_string(),
			Protocol::Ipv4 => "IP address".to_string(),
		}
	}
}

impl FromStr for Protocol {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"hostname" | "dns" => Ok(Protocol::Dns),
			"IP address" | "ip4" => Ok(Protocol::Ipv4),
			_ => Err("Invalid protocol".to_string()),
		}
	}
}

const INTRO: &str =
	"This 🧙 wizard will:
* establish single bootnode configuration (to be later included in chain-spec file)
* choose Genesis UTXO on Cardano
* initialize Partner Chains Governance on Cardano
* establish Partner Chains Smart Contracts addresses and policies (to be later included in chain-spec file)";

fn outro(chain_config_file_path: String) -> String {
	format!(
		"Bootnode saved successfully. Keep in mind that you can manually modify {}, to edit bootnodes.",
		chain_config_file_path
	)
}

const CHOOSE_PROTOCOL_PROMPT: &str = "Your bootnode should be accessible via:";
const CHOOSE_PORT_PROMPT: &str = "Enter bootnode TCP port";
const DEFAULT_PORT: u16 = 3033;

const CHOOSE_HOSTNAME_PROMPT: &str = "Enter bootnode hostname";

const CHOOSE_IP_ADDRESS_PROMPT: &str = "Enter bootnode IP address";

fn dns_bootnode(hostname: &str, port: u16, peer_id: &str) -> String {
	format!("/dns/{}/tcp/{}/p2p/{}", hostname, port, peer_id)
}

fn ipv4_bootnode(ip_address: &str, port: u16, peer_id: &str) -> String {
	format!("/ip4/{}/tcp/{}/p2p/{}", ip_address, port, peer_id)
}

fn peer_id_from_network_key(key_str: &str) -> anyhow::Result<String> {
	let hex_vec = hex::decode(key_str).context("Invalid hex")?;
	let keypair = Keypair::ed25519_from_bytes(hex_vec).context("Invalid Ed25519 bytes")?;
	Ok(keypair.public().to_peer_id().to_base58())
}

impl<T> InitGovernance for T
where
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
{
	async fn init_governance(
		&self,
		await_tx: FixedDelayRetries,
		governance_parameters: &MultiSigParameters,
		payment_key: &CardanoPaymentSigningKey,
		genesis_utxo_id: UtxoId,
	) -> Result<McTxHash, String> {
		partner_chains_cardano_offchain::init_governance::run_init_governance(
			governance_parameters,
			payment_key,
			Some(genesis_utxo_id),
			self,
			await_tx,
		)
		.await
		.map(|result| result.tx_hash)
		.map_err(|e| e.to_string())
	}
}

impl<T: QueryNetwork> GetScriptsData for T {
	async fn get_scripts_data(&self, genesis_utxo: UtxoId) -> Result<ScriptsData, String> {
		let network = self
			.shelley_genesis_configuration()
			.await
			.map_err(|e| format!("Ogmios error: {e}"))?
			.network
			.to_csl();
		get_scripts_data(genesis_utxo, network).map_err(|e| e.to_string())
	}
}

#[cfg(test)]
pub mod tests {
	use super::*;
	use crate::config::{ConfigFieldDefinition, SelectOptions};
	use crate::prepare_configuration::Protocol::{Dns, Ipv4};
	use crate::tests::{CHAIN_CONFIG_FILE_PATH, MockIO, MockIOContext, RESOURCES_CONFIG_FILE_PATH};
	use crate::{CommonArguments, verify_json};

	const KEY: &str = "962515971a22aa95706c2109ba6e9502c7f39b33bdf63024f46f77894424f1fe";
	pub const DATA_PATH: &str = "/path/to/data";

	fn network_key_file() -> String {
		format!("{DATA_PATH}/network/secret_ed25519")
	}

	pub mod scenarios {
		use super::*;
		use crate::config::config_fields::SUBSTRATE_NODE_DATA_BASE_PATH;
		use crate::prepare_configuration::Protocol::{Dns, Ipv4};

		pub fn show_intro() -> MockIO {
			MockIO::Group(vec![MockIO::eprint(
				"This 🧙 wizard will:
* establish single bootnode configuration (to be later included in chain-spec file)
* choose Genesis UTXO on Cardano
* initialize Partner Chains Governance on Cardano
* establish Partner Chains Smart Contracts addresses and policies (to be later included in chain-spec file)",
			)])
		}

		pub fn read_config() -> MockIO {
			MockIO::Group(vec![prompt_with_default(
				SUBSTRATE_NODE_DATA_BASE_PATH,
				SUBSTRATE_NODE_DATA_BASE_PATH.default,
				DATA_PATH,
			)])
		}

		pub fn choose_to_configure_bootnode(choice: bool) -> MockIO {
			MockIO::PromptYN {
				prompt: "Do you want to configure a single bootnode with your peer id '12D3KooWSi9ys81fpG9ibuVWh6w6egfcTUM8L1iSJSpfFtMLMLG9'?"
					.to_string(),
				default: true,
				choice,
			}
		}

		pub fn pick_ip_protocol_with_defaults() -> MockIO {
			pick_ip_protocol(
				vec![Dns.into(), Ipv4.into()],
				DEFAULT_PORT,
				Ipv4.default_address().to_string(),
			)
		}

		pub fn pick_ip_protocol(
			options: Vec<String>,
			default_port: u16,
			default_ip_address: String,
		) -> MockIO {
			pick_chosen_ip_protocol(options, default_port, &default_ip_address, "10.2.2.4")
		}

		pub fn pick_chosen_ip_protocol(
			options: Vec<String>,
			default_port: u16,
			default_ip_address: &str,
			input: &str,
		) -> MockIO {
			MockIO::Group(vec![
				MockIO::prompt_multi_option(CHOOSE_PROTOCOL_PROMPT, options, &String::from(Ipv4)),
				MockIO::prompt(
					CHOOSE_PORT_PROMPT,
					Some(&default_port.to_string()),
					&default_port.to_string(),
				),
				MockIO::prompt(CHOOSE_IP_ADDRESS_PROMPT, Some(default_ip_address), input),
			])
		}

		pub fn ip_bootnode(key: &str, port: u16) -> serde_json::Value {
			let peer_id = peer_id_from_network_key(key).unwrap();
			serde_json::json!({
				"bootnodes": [format!("/ip4/10.2.2.4/tcp/{port}/p2p/{peer_id}")]
			})
		}

		pub fn pick_dns_protocol_with_defaults() -> MockIO {
			pick_dns_protocol(
				vec![Dns.into(), Ipv4.into()],
				DEFAULT_PORT,
				Dns.default_address().to_string(),
			)
		}

		pub fn pick_dns_protocol(
			options: Vec<String>,
			default_port: u16,
			default_hostname: String,
		) -> MockIO {
			MockIO::Group(vec![
				MockIO::prompt_multi_option(CHOOSE_PROTOCOL_PROMPT, options, &String::from(Dns)),
				MockIO::prompt(
					CHOOSE_PORT_PROMPT,
					Some(&default_port.to_string()),
					&default_port.to_string(),
				),
				MockIO::prompt(CHOOSE_HOSTNAME_PROMPT, Some(&default_hostname), "iog.io"),
			])
		}

		pub fn dns_bootnode(key: &str, port: u16) -> serde_json::Value {
			let peer_id = peer_id_from_network_key(key).unwrap();
			serde_json::json!({
				"bootnodes": [format!("/dns/iog.io/tcp/{port}/p2p/{peer_id}")]
			})
		}
	}

	fn context_with_config(key: &str) -> MockIOContext {
		MockIOContext::new()
			.with_file(&network_key_file(), key)
			.with_json_file(RESOURCES_CONFIG_FILE_PATH, serde_json::json!({}))
	}

	#[test]
	fn peer_id_correctly_generated_from_secret_network_key() {
		let peer_id = peer_id_from_network_key(KEY).unwrap();
		assert_eq!(peer_id, "12D3KooWSi9ys81fpG9ibuVWh6w6egfcTUM8L1iSJSpfFtMLMLG9");
	}

	#[test]
	fn happy_path_ip() {
		let mock_context = context_with_config(KEY).with_expected_io(vec![
			scenarios::read_config(),
			scenarios::choose_to_configure_bootnode(true),
			scenarios::pick_ip_protocol_with_defaults(),
			MockIO::eprint(&outro(CHAIN_CONFIG_FILE_PATH.to_owned())),
		]);

		let result = establish_bootnodes(&mock_context);
		verify_json!(
			mock_context,
			CHAIN_CONFIG_FILE_PATH,
			scenarios::ip_bootnode(KEY, DEFAULT_PORT)
		);

		result.expect("should succeed");
	}

	#[test]
	fn happy_path_hostname() {
		let mock_context = context_with_config(KEY).with_expected_io(vec![
			scenarios::read_config(),
			scenarios::choose_to_configure_bootnode(true),
			scenarios::pick_dns_protocol_with_defaults(),
			MockIO::eprint(&outro(CHAIN_CONFIG_FILE_PATH.to_owned())),
		]);

		let result = establish_bootnodes(&mock_context);

		result.expect("should succeed");
		verify_json!(
			mock_context,
			CHAIN_CONFIG_FILE_PATH,
			scenarios::dns_bootnode(KEY, DEFAULT_PORT)
		);
	}

	#[test]
	fn happy_path_no_bootnode() {
		let mock_context = context_with_config(KEY).with_expected_io(vec![
			scenarios::read_config(),
			scenarios::choose_to_configure_bootnode(false),
		]);
		let result = establish_bootnodes(&mock_context);
		result.expect("should succeed");
		verify_json!(mock_context, CHAIN_CONFIG_FILE_PATH, serde_json::json!({"bootnodes": []}));
	}

	#[test]
	fn propose_saved_defaults_but_pick_different() {
		let mock_context = context_with_config(KEY)
			.with_json_file(
				CHAIN_CONFIG_FILE_PATH,
				serde_json::json!({
					"bootnodes": ["/ip4/ip_address/tcp/3034/p2p/12D3KooWWi9ys81fpG9ibuVWh6w6egfcTUM8L1iSJSpfFtMLMLG8"]
				}),
			)
			.with_expected_io(vec![
				scenarios::read_config(),
				scenarios::choose_to_configure_bootnode(true),
				scenarios::pick_dns_protocol(
					vec![Ipv4.into(), Dns.into()],
					3034,
					Dns.default_address().to_string(),
				),
				MockIO::eprint(&outro(CHAIN_CONFIG_FILE_PATH.to_owned())),
			]);

		let result = establish_bootnodes(&mock_context);
		verify_json!(mock_context, CHAIN_CONFIG_FILE_PATH, scenarios::dns_bootnode(KEY, 3034));

		result.expect("should succeed");
	}

	#[test]
	fn propose_saved_defaults_and_pick_it() {
		let mock_context = context_with_config(KEY)
			.with_json_file(
				CHAIN_CONFIG_FILE_PATH,
				serde_json::json!({
					"bootnodes": ["/ip4/ip_address/tcp/3034/p2p/12D3KooWWi9ys81fpG9ibuVWh6w6egfcTUM8L1iSJSpfFtMLMLG8"]
				}),
			)
			.with_expected_io(vec![
				scenarios::read_config(),
				scenarios::choose_to_configure_bootnode(true),
				scenarios::pick_ip_protocol(
					vec![Ipv4.into(), Dns.into()],
					3034,
					"ip_address".to_string(),
				),
				MockIO::eprint(&outro(CHAIN_CONFIG_FILE_PATH.to_owned())),
			]);

		let result = establish_bootnodes(&mock_context);
		verify_json!(mock_context, CHAIN_CONFIG_FILE_PATH, scenarios::ip_bootnode(KEY, 3034));

		result.expect("should succeed");
	}

	#[test]
	fn continue_without_network_key_file_when_user_agrees() {
		let mock_context = MockIOContext::new()
			.with_json_file(CHAIN_CONFIG_FILE_PATH, serde_json::json!({}))
			.with_expected_io(vec![
				scenarios::read_config(),
				MockIO::print(&format!("Could not read network key from {}", network_key_file())),
				MockIO::prompt_yes_no(
					"Could not read network secret key from the base directory. Do you want to continue without configuring bootnode?",
					true,
					true,
				),
			]);

		let result = establish_bootnodes(&mock_context);
		result.expect("should succeed");
		verify_json!(mock_context, CHAIN_CONFIG_FILE_PATH, serde_json::json!({"bootnodes": []}));
	}

	#[test]
	fn do_not_error_without_resources_config_file() {
		let mock_context =
			MockIOContext::new().with_file(&network_key_file(), KEY).with_expected_io(vec![
				scenarios::read_config(),
				scenarios::choose_to_configure_bootnode(true),
				scenarios::pick_ip_protocol_with_defaults(),
				MockIO::eprint(&outro(CHAIN_CONFIG_FILE_PATH.to_owned())),
			]);

		let result = establish_bootnodes(&mock_context);

		result.expect("should succeed");
		verify_json!(
			mock_context,
			RESOURCES_CONFIG_FILE_PATH,
			serde_json::json!({"substrate_node_base_path": DATA_PATH})
		);
		verify_json!(
			mock_context,
			CHAIN_CONFIG_FILE_PATH,
			scenarios::ip_bootnode(KEY, DEFAULT_PORT)
		);
	}

	#[test]
	fn dont_accept_invalid_ip_address() {
		let mock_context = context_with_config(KEY).with_expected_io(vec![
			scenarios::show_intro(),
			scenarios::read_config(),
			scenarios::choose_to_configure_bootnode(true),
			scenarios::pick_chosen_ip_protocol(
				vec![Dns.into(), Ipv4.into()],
				DEFAULT_PORT,
				Ipv4.default_address(),
				"100",
			),
		]);

		let result =
			PrepareConfigurationCmd { common_arguments: common_arguments() }.run(&mock_context);

		let error = result.expect_err("should return error");
		assert!(error.to_string().contains("⚠️ Invalid IP address"));
	}

	fn common_arguments() -> CommonArguments {
		CommonArguments { retry_delay_seconds: 5, retry_count: 59 }
	}

	pub fn prompt<T>(field_definition: ConfigFieldDefinition<'_, T>, value: &str) -> MockIO {
		let default = field_definition.default;
		prompt_with_default(field_definition, default, value)
	}

	pub fn prompt_with_default<T>(
		field_definition: ConfigFieldDefinition<'_, T>,
		default: Option<&str>,
		value: &str,
	) -> MockIO {
		MockIO::prompt(&format!("Enter the {}", field_definition.name), default, value)
	}

	pub fn prompt_multi_option_with_default<T: SelectOptions>(
		field_definition: ConfigFieldDefinition<'_, T>,
		default: Option<&str>,
		value: &str,
	) -> MockIO {
		MockIO::prompt_multi_option(
			&format!("Select {}", field_definition.name),
			T::select_options_with_default(default),
			value,
		)
	}
}
