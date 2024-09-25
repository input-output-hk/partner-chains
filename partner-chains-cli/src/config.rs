use crate::config::config_fields::{
	CARDANO_ACTIVE_SLOTS_COEFF, CARDANO_EPOCH_DURATION_MILLIS, CARDANO_FIRST_EPOCH_NUMBER,
	CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS, CARDANO_FIRST_SLOT_NUMBER, CARDANO_SECURITY_PARAMETER,
};
use crate::io::IOContext;
use anyhow::anyhow;
use clap::{arg, Parser};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sidechain_domain::{MainchainAddressHash, UtxoId};
use sp_core::offchain::{Duration, Timestamp};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{marker::PhantomData, process::exit};

pub struct ConfigFieldDefinition<'a, T> {
	pub name: &'a str,
	pub config_file: &'a str,
	pub path: &'a [&'a str],
	pub default: Option<&'a str>,
	pub _marker: PhantomData<T>,
}

impl<'a> ConfigFieldDefinition<'a, String> {
	pub fn load_or_prompt_and_save<C: IOContext>(&self, context: &C) -> String {
		if let Some(value) = self.load_from_file_and_print(context) {
			value
		} else {
			let value = context.prompt(self.name, self.default);
			self.save_to_file(&value, context);
			value
		}
	}

	pub fn prompt_with_default_from_file_and_save<C: IOContext>(&self, context: &C) -> String {
		let value =
			context.prompt(self.name, self.load_from_file(context).as_deref().or(self.default));
		self.save_to_file(&value, context);
		value
	}
}

impl<'a, T> ConfigFieldDefinition<'a, T> {
	pub fn prompt_with_default_from_file_parse_and_save<C: IOContext>(
		&self,
		context: &C,
	) -> Result<T, <T as FromStr>::Err>
	where
		T: DeserializeOwned + std::fmt::Display + FromStr + serde::Serialize,
	{
		let loaded_value = self.load_from_file(context).map(|v| v.to_string());
		let default_value = loaded_value.as_deref().or(self.default);
		let value = context.prompt(self.name, default_value);
		let parsed_value: T = value.parse()?;
		self.save_to_file(&parsed_value, context);
		Ok(parsed_value)
	}

	pub fn select_options_with_default_from_file_and_save<C: IOContext>(
		&self,
		prompt: &str,
		context: &C,
	) -> Result<T, <T as FromStr>::Err>
	where
		T: DeserializeOwned + std::fmt::Display + FromStr + serde::Serialize + SelectOptions,
	{
		let loaded_value = self.load_from_file(context).map(|v| v.to_string());
		let default_value_opt = loaded_value.as_deref().or(self.default);
		let options = T::select_options_with_default(default_value_opt);
		let value = context.prompt_multi_option(prompt, options);
		let parsed_value: T = value.parse()?;
		self.save_to_file(&parsed_value, context);
		Ok(parsed_value)
	}

	pub fn load_or_prompt_parse_and_save<C: IOContext>(
		&self,
		context: &C,
	) -> Result<T, <T as FromStr>::Err>
	where
		T: DeserializeOwned + std::fmt::Display + FromStr + serde::Serialize,
	{
		if let Some(value) = self.load_from_file_and_print(context) {
			Ok(value)
		} else {
			let value_str = context.prompt(self.name, self.default);
			let parsed_value: T = value_str.parse()?;
			self.save_to_file(&parsed_value, context);
			Ok(parsed_value)
		}
	}

	/// loads and parses the config field
	pub fn load_from_file<C: IOContext>(&self, context: &C) -> Option<T>
	where
		T: DeserializeOwned,
	{
		self.load_file(context).and_then(|json| self.extract_from_json_object(&json))
	}

	pub fn load_from_file_and_print(&self, context: &impl IOContext) -> Option<T>
	where
		T: DeserializeOwned + std::fmt::Display,
	{
		let value = self.load_from_file(context)?;
		context.eprint(&self.loaded_from_config_msg(&value));
		Some(value)
	}

	/// updates the config field in the file
	pub fn save_to_file<C: IOContext>(&self, value: &T, context: &C)
	where
		T: Serialize,
	{
		let mut json =
			self.load_file(context).unwrap_or(serde_json::Value::Object(Default::default()));
		let mut head = &mut json;
		for &field in self.path {
			head[field] = head
				.get(field)
				.cloned()
				.filter(serde_json::Value::is_object)
				.unwrap_or(serde_json::Value::Object(Default::default()));
			head = &mut head[field];
		}
		*head = serde_json::to_value(value).unwrap();
		context.write_file(self.config_file, &serde_json::to_string_pretty(&json).unwrap());
	}

	pub fn save_if_empty<C: IOContext>(&self, value: T, context: &C) -> T
	where
		T: DeserializeOwned + serde::Serialize,
	{
		if let Some(value) = self.load_from_file(context) {
			value
		} else {
			self.save_to_file(&value, context);
			value
		}
	}

	/// parses the config field's type from a json value
	pub fn extract_from_json_object(&self, json: &serde_json::Value) -> Option<T>
	where
		T: DeserializeOwned,
	{
		let mut json: Option<&serde_json::Value> = Some(json);
		for &field in self.path {
			if let Some(json_inner) = json {
				json = json_inner.get(field)
			} else {
				return None;
			}
		}
		json.and_then(|json| serde_json::from_value(json.clone()).ok())
	}

	/// loads the whole content of the config fields relevant config file
	pub fn load_file<C: IOContext>(&self, context: &C) -> Option<serde_json::Value> {
		if !context.file_exists(self.config_file) {
			return None;
		}

		if let Some(file_content_string) = context.read_file(self.config_file) {
			if let Ok(value) = serde_json::from_str(&file_content_string) {
				return Some(value);
			}
		}

		self.report_corrupted_file_and_quit()
	}

	/// print error message and exit
	pub fn report_corrupted_file_and_quit(&self) -> ! {
		eprintln!(
			"Config file {} is broken. Delete it or fix manually and restart this wizard",
			self.config_file
		);
		exit(-1)
	}

	pub fn loaded_from_config_msg(&self, value: &T) -> String
	where
		T: std::fmt::Display,
	{
		format!("🛠️ Loaded {} from config ({}): {value}", self.name, self.config_file)
	}

	pub fn json_pointer(&self) -> String {
		format!("/{}", self.path.join("/"))
	}
}

#[derive(Clone, Debug)]
pub struct ServiceConfig {
	pub protocol: NetworkProtocol,
	pub hostname: String,
	pub port: u16,
}

impl Display for ServiceConfig {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		f.write_str(&format!("{}://{}:{}", self.protocol, self.hostname, self.port))
	}
}

pub trait SelectOptions {
	fn select_options() -> Vec<String>;
	fn select_options_with_default(default_value_opt: Option<&str>) -> Vec<String> {
		let mut options = Self::select_options();

		if let Some(default_value) = default_value_opt {
			options.sort_by_key(|option| if *option != default_value { 1 } else { 0 });
		}
		options
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NetworkProtocol {
	#[serde(rename = "http")]
	Http,
	#[serde(rename = "https")]
	Https,
}

impl NetworkProtocol {
	pub fn is_secure(&self) -> bool {
		matches!(self, NetworkProtocol::Https)
	}
}

impl std::fmt::Display for NetworkProtocol {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let str = match self {
			NetworkProtocol::Http => "http".to_string(),
			NetworkProtocol::Https => "https".to_string(),
		};
		write!(f, "{}", str)
	}
}

impl FromStr for NetworkProtocol {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"http" => Ok(NetworkProtocol::Http),
			"https" => Ok(NetworkProtocol::Https),
			other => {
				Err(format!("Invalid security protocol {other}, please provide http or https"))
			},
		}
	}
}

impl SelectOptions for NetworkProtocol {
	fn select_options() -> Vec<String> {
		vec![NetworkProtocol::Http.to_string(), NetworkProtocol::Https.to_string()]
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CardanoNetwork(pub u32);

impl CardanoNetwork {
	pub fn to_id(&self) -> u32 {
		self.0
	}
	pub fn to_network_param(&self) -> String {
		match self {
			CardanoNetwork(0) => "mainnet".into(),
			_ => "testnet".into(),
		}
	}
}

impl FromStr for CardanoNetwork {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"mainnet" => Ok(CardanoNetwork(0)),
			"preprod" => Ok(CardanoNetwork(1)),
			"preview" => Ok(CardanoNetwork(2)),
			_ => Ok(CardanoNetwork(3)),
		}
	}
}

impl std::fmt::Display for CardanoNetwork {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			CardanoNetwork(0) => write!(f, "mainnet"),
			CardanoNetwork(1) => write!(f, "preprod"),
			CardanoNetwork(2) => write!(f, "preview"),
			CardanoNetwork(_) => write!(f, "custom"),
		}
	}
}

impl SelectOptions for CardanoNetwork {
	fn select_options() -> Vec<String> {
		vec![
			CardanoNetwork(0).to_string(),
			CardanoNetwork(1).to_string(),
			CardanoNetwork(2).to_string(),
			CardanoNetwork(3).to_string(),
		]
	}
}

#[derive(Deserialize)]
pub struct MainChainAddresses {
	pub committee_candidates_address: String,
	pub d_parameter_policy_id: String,
	pub permissioned_candidates_policy_id: String,
	pub native_token: NativeTokenConfig,
}
#[derive(Deserialize, PartialEq, Clone, Debug)]
pub struct CardanoParameters {
	pub security_parameter: u64,
	pub active_slots_coeff: f64,
	pub first_epoch_number: u32,
	pub first_slot_number: u64,
	pub epoch_duration_millis: u64,
	pub first_epoch_timestamp_millis: u64,
}

impl CardanoParameters {
	pub fn save(&self, context: &impl IOContext) {
		CARDANO_SECURITY_PARAMETER.save_to_file(&self.security_parameter, context);
		CARDANO_ACTIVE_SLOTS_COEFF.save_to_file(&self.active_slots_coeff, context);
		CARDANO_FIRST_EPOCH_NUMBER.save_to_file(&self.first_epoch_number, context);
		CARDANO_FIRST_SLOT_NUMBER.save_to_file(&self.first_slot_number, context);
		CARDANO_EPOCH_DURATION_MILLIS.save_to_file(&self.epoch_duration_millis, context);
		CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS
			.save_to_file(&self.first_epoch_timestamp_millis, context);
	}

	pub fn read(context: &impl IOContext) -> Option<Self> {
		Some(Self {
			security_parameter: CARDANO_SECURITY_PARAMETER.load_from_file(context)?,
			active_slots_coeff: CARDANO_ACTIVE_SLOTS_COEFF.load_from_file(context)?,
			first_epoch_number: CARDANO_FIRST_EPOCH_NUMBER.load_from_file(context)?,
			first_slot_number: CARDANO_FIRST_SLOT_NUMBER.load_from_file(context)?,
			epoch_duration_millis: CARDANO_EPOCH_DURATION_MILLIS.load_from_file(context)?,
			first_epoch_timestamp_millis: CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS
				.load_from_file(context)?,
		})
	}
}

impl From<CardanoParameters> for epoch_derivation::MainchainEpochConfig {
	fn from(value: CardanoParameters) -> Self {
		Self {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(
				value.first_epoch_timestamp_millis,
			),
			epoch_duration_millis: Duration::from_millis(value.epoch_duration_millis),
			first_epoch_number: value.first_epoch_number,
			first_slot_number: value.first_slot_number,
		}
	}
}

#[derive(Deserialize, Serialize, Parser, Clone, Debug)]
pub struct SidechainParams {
	#[arg(long)]
	pub chain_id: u16,
	#[arg(long)]
	pub genesis_committee_utxo: UtxoId,
	#[arg(long)]
	pub threshold_numerator: u64,
	#[arg(long)]
	pub threshold_denominator: u64,
	#[arg(long)]
	pub governance_authority: MainchainAddressHash,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct AssetConfig {
	policy_id: String,
	asset_name: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NativeTokenConfig {
	pub asset: AssetConfig,
	pub illiquid_supply_address: String,
}

#[derive(Deserialize)]
pub struct ChainConfig {
	pub cardano: CardanoParameters,
	pub chain_parameters: SidechainParams,
	pub cardano_addresses: MainChainAddresses,
}

pub const KEYS_FILE_PATH: &str = "partner-chains-public-keys.json";
pub const CHAIN_CONFIG_FILE_PATH: &str = "partner-chains-cli-chain-config.json";
pub const RESOURCES_CONFIG_FILE_PATH: &str = "partner-chains-cli-resources-config.json";
pub const CHAIN_SPEC_PATH: &str = "chain-spec.json";
pub const PC_CONTRACTS_CLI_PATH: &str = "./pc-contracts-cli";

pub fn load_chain_config(context: &impl IOContext) -> anyhow::Result<ChainConfig> {
	if let Some(chain_config_file) = context.read_file(CHAIN_CONFIG_FILE_PATH) {
		serde_json::from_str::<ChainConfig>(&chain_config_file)
			.map_err(|err| anyhow::anyhow!(format!("⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} is invalid: {err}. Run prepare-configuration wizard or fix errors manually.")))
	} else {
		Err(anyhow::anyhow!(format!("⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} does not exists. Run prepare-configuration wizard first.")))
	}
}

pub fn get_cardano_network_from_file(context: &impl IOContext) -> anyhow::Result<CardanoNetwork> {
	config_fields::CARDANO_NETWORK.load_from_file(context).ok_or(anyhow!(
		"Cardano network not configured. Please run prepare-main-chain-config command first."
	))
}

pub mod config_fields {
	use super::*;
	use sidechain_domain::{MainchainAddressHash, UtxoId};

	pub const NATIVE_TOKEN_POLICY: ConfigFieldDefinition<'static, String> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["cardano_addresses", "native_token", "asset", "policy_id"],
		name: "native token policy ID",
		default: None,
		_marker: PhantomData,
	};

	pub const NATIVE_TOKEN_ASSET_NAME: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "native_token", "asset", "asset_name"],
			name: "native token asset name in hex",
			default: None,
			_marker: PhantomData,
		};

	pub const ILLIQUID_SUPPLY_ADDRESS: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "native_token", "illiquid_supply_address"],
			name: "native token illiquid token supply address",
			default: None,
			_marker: PhantomData,
		};

	pub const SUBSTRATE_NODE_DATA_BASE_PATH: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["substrate_node_base_path"],
			name: "node base path",
			default: Some("./data"),
			_marker: PhantomData,
		};

	pub const NODE_EXECUTABLE_DEFAULT: &str = "./partner-chains-node";

	pub const NODE_EXECUTABLE: ConfigFieldDefinition<'static, String> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["substrate_node_executable_path"],
		name: "Partner Chains node executable",
		default: Some(NODE_EXECUTABLE_DEFAULT),
		_marker: PhantomData,
	};

	pub const CARDANO_CLI: ConfigFieldDefinition<'static, String> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["cardano_cli"],
		name: "cardano cli executable",
		default: Some("cardano-cli"),
		_marker: PhantomData,
	};

	pub const CARDANO_NODE_SOCKET_PATH: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["cardano_node_socket_path"],
			name: "path to the cardano node socket file",
			default: Some("node.socket"),
			_marker: PhantomData,
		};

	pub const CARDANO_PAYMENT_VERIFICATION_KEY_FILE: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["cardano_payment_verification_key_file"],
			name: "path to the payment verification file",
			default: Some("payment.vkey"),
			_marker: PhantomData,
		};

	pub const CARDANO_PAYMENT_SIGNING_KEY_FILE: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["cardano_payment_signing_key_file"],
			name: "path to the payment signing file",
			default: Some("payment.skey"),
			_marker: PhantomData,
		};

	pub const CARDANO_NETWORK: ConfigFieldDefinition<'static, CardanoNetwork> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "network"],
			name: "cardano network",
			default: Some("0"),
			_marker: PhantomData,
		};

	pub const CHAIN_ID: ConfigFieldDefinition<'static, u16> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["chain_parameters", "chain_id"],
		name: "partner chain id",
		default: Some("0"),
		_marker: PhantomData,
	};

	pub const THRESHOLD_NUMERATOR: ConfigFieldDefinition<'static, u64> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["chain_parameters", "threshold_numerator"],
		name: "threshold numerator",
		default: Some("2"),
		_marker: PhantomData,
	};

	pub const THRESHOLD_DENOMINATOR: ConfigFieldDefinition<'static, u64> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["chain_parameters", "threshold_denominator"],
		name: "threshold denominator",
		default: Some("3"),
		_marker: PhantomData,
	};

	pub const GOVERNANCE_AUTHORITY: ConfigFieldDefinition<'static, MainchainAddressHash> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["chain_parameters", "governance_authority"],
			name: "governance authority",
			default: None,
			_marker: PhantomData,
		};

	pub const GENESIS_COMMITTEE_UTXO: ConfigFieldDefinition<'static, UtxoId> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["chain_parameters", "genesis_committee_utxo"],
			name: "genesis committee utxo",
			default: Some("0000000000000000000000000000000000000000000000000000000000000000#0"),
			_marker: PhantomData,
		};

	pub const BOOTNODES: ConfigFieldDefinition<'static, Vec<String>> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["bootnodes"],
		name: "bootnodes",
		default: None,
		_marker: PhantomData,
	};

	pub(crate) const INITIAL_PERMISSIONED_CANDIDATES: ConfigFieldDefinition<
		'static,
		Vec<crate::permissioned_candidates::PermissionedCandidateKeys>,
	> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["initial_permissioned_candidates"],
		name: "initial permissioned candidates",
		default: None,
		_marker: PhantomData,
	};

	pub const POSTGRES_CONNECTION_STRING: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["db_sync_postgres_connection_string"],
			name: "DB-Sync Postgres connection string",
			default: Some("postgresql://postgres-user:postgres-password@localhost:5432/cexplorer"),
			_marker: PhantomData,
		};

	pub const KUPO_PROTOCOL: ConfigFieldDefinition<'static, NetworkProtocol> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["kupo", "protocol"],
			name: "Kupo protocol (http/https)",
			default: Some("http"),
			_marker: PhantomData,
		};

	pub const KUPO_HOSTNAME: ConfigFieldDefinition<'static, String> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["kupo", "hostname"],
		name: "Kupo hostname",
		default: Some("localhost"),
		_marker: PhantomData,
	};

	pub const KUPO_PORT: ConfigFieldDefinition<'static, u16> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["kupo", "port"],
		name: "Kupo port",
		default: Some("1442"),
		_marker: PhantomData,
	};

	pub const OGMIOS_PROTOCOL: ConfigFieldDefinition<'static, NetworkProtocol> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["ogmios", "protocol"],
			name: "Ogmios protocol (http/https)",
			default: Some("http"),
			_marker: PhantomData,
		};

	pub const OGMIOS_HOSTNAME: ConfigFieldDefinition<'static, String> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["ogmios", "hostname"],
		name: "Ogmios hostname",
		default: Some("localhost"),
		_marker: PhantomData,
	};

	pub const OGMIOS_PORT: ConfigFieldDefinition<'static, u16> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["ogmios", "port"],
		name: "Ogmios port",
		default: Some("1337"),
		_marker: PhantomData,
	};

	pub const COMMITTEE_CANDIDATES_ADDRESS: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "committee_candidates_address"],
			name: "Committee candidates address",
			default: None,
			_marker: PhantomData,
		};

	pub const D_PARAMETER_POLICY_ID: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "d_parameter_policy_id"],
			name: "D parameter policy id",
			default: None,
			_marker: PhantomData,
		};

	pub const PERMISSIONED_CANDIDATES_POLICY_ID: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "permissioned_candidates_policy_id"],
			name: "permissioned candidates policy id",
			default: None,
			_marker: PhantomData,
		};

	pub const CARDANO_SECURITY_PARAMETER: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "security_parameter"],
			name: "cardano security parameter",
			default: None,
			_marker: PhantomData,
		};

	pub const CARDANO_ACTIVE_SLOTS_COEFF: ConfigFieldDefinition<'static, f64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "active_slots_coeff"],
			name: "cardano active slot coefficient",
			default: None,
			_marker: PhantomData,
		};

	pub const CARDANO_FIRST_EPOCH_NUMBER: ConfigFieldDefinition<'static, u32> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "first_epoch_number"],
			name: "cardano first epoch number in shelley era",
			default: None,
			_marker: PhantomData,
		};

	pub const CARDANO_FIRST_SLOT_NUMBER: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "first_slot_number"],
			name: "cardano first slot number in shelley era",
			default: None,
			_marker: PhantomData,
		};

	pub const CARDANO_EPOCH_DURATION_MILLIS: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "epoch_duration_millis"],
			name: "cardano epoch duration in millis",
			default: None,
			_marker: PhantomData,
		};

	pub const CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "first_epoch_timestamp_millis"],
			name: "cardano first shelley epoch timestamp in millis",
			default: None,
			_marker: PhantomData,
		};

	pub const NODE_P2P_PORT: ConfigFieldDefinition<'static, u16> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["node_p2p_port"],
		name: "substrate-node p2p protocol TCP port",
		default: Some("30333"),
		_marker: PhantomData,
	};

	pub const SIDECHAIN_BLOCK_BENEFICIARY: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["sidechain_block_beneficiary"],
			name: "beneficiary for blocks created by the given node",
			default: None,
			_marker: PhantomData,
		};
}

pub mod config_values {
	pub const DEFAULT_CHAIN_NAME: &str = "partner_chains_template";
}

#[cfg(test)]
mod tests {
	use crate::config::config_fields::GOVERNANCE_AUTHORITY;
	use sidechain_domain::MainchainAddressHash;

	#[test]
	fn governance_authority_without_leading_0x() {
		let parsed = GOVERNANCE_AUTHORITY.extract_from_json_object(&serde_json::json!({"chain_parameters":{"governance_authority":"000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"}}));
		assert_eq!(
			parsed,
			Some(MainchainAddressHash::from_hex_unsafe(
				"000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
			))
		);
	}

	#[test]
	fn governance_authority_with_leading_0x() {
		let parsed = GOVERNANCE_AUTHORITY.extract_from_json_object(&serde_json::json!({"chain_parameters":{"governance_authority":"0x000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"}}));
		assert_eq!(
			parsed,
			Some(MainchainAddressHash::from_hex_unsafe(
				"000000b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
			))
		);
	}
}
