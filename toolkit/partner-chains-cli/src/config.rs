use crate::config::config_fields::{
	CARDANO_ACTIVE_SLOTS_COEFF, CARDANO_EPOCH_DURATION_MILLIS, CARDANO_FIRST_EPOCH_NUMBER,
	CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS, CARDANO_FIRST_SLOT_NUMBER, CARDANO_SECURITY_PARAMETER,
};
use crate::io::IOContext;
use clap::{Parser, arg};
use config_fields::CARDANO_SLOT_DURATION_MILLIS;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sidechain_domain::{MainchainKeyHash, UtxoId};
use sp_core::offchain::{Duration, Timestamp};
use std::fmt::{Display, Formatter, Write};
use std::str::FromStr;
use std::{marker::PhantomData, process::exit};

/// Definition of a field in a config JSON with optional default value.
pub(crate) struct ConfigFieldDefinition<'a, T> {
	/// Config field name
	pub(crate) name: &'a str,
	/// Config file name
	pub(crate) config_file: &'a str,
	/// Path to config field in config file
	pub(crate) path: &'a [&'a str],
	/// Optional default value for config field
	pub(crate) default: Option<&'a str>,
	_marker: PhantomData<T>,
}

#[cfg(test)]
impl<'a, T> ConfigFieldDefinition<'a, T> {
	pub(crate) fn new(
		name: &'a str,
		config_file: &'a str,
		path: &'a [&'a str],
		default: Option<&'a str>,
	) -> Self {
		Self { name, config_file, path, default, _marker: Default::default() }
	}
}

impl<'a> ConfigFieldDefinition<'a, String> {
	/// Tries to load the config field value, as a string, from its expected location, and
	/// if not found it prompts the user for it.
	pub(crate) fn load_or_prompt_and_save<C: IOContext>(&self, context: &C) -> String {
		if let Some(value) = self.load_from_file_and_print(context) {
			value
		} else {
			let value = context.prompt(&format!("Enter the {}", self.name), self.default);
			self.save_to_file(&value, context);
			value
		}
	}

	/// Prompts for the config field value, as a string, with a default if it is set in [ConfigFieldDefinition].
	pub(crate) fn prompt_with_default_from_file_and_save<C: IOContext>(
		&self,
		context: &C,
	) -> String {
		let value = context.prompt(
			&format!("Enter the {}", self.name),
			self.load_from_file(context).as_deref().or(self.default),
		);
		self.save_to_file(&value, context);
		value
	}
}

impl<'a, T> ConfigFieldDefinition<'a, T> {
	pub(crate) fn prompt_with_default_from_file_parse_and_save<C: IOContext>(
		&self,
		context: &C,
	) -> Result<T, <T as FromStr>::Err>
	where
		T: DeserializeOwned + std::fmt::Display + FromStr + serde::Serialize,
	{
		let loaded_value = self.load_from_file(context).map(|v| v.to_string());
		let default_value = loaded_value.as_deref().or(self.default);
		let value = context.prompt(&format!("Enter the {}", self.name), default_value);
		let parsed_value: T = value.parse()?;
		self.save_to_file(&parsed_value, context);
		Ok(parsed_value)
	}

	pub(crate) fn select_options_with_default_from_file_and_save<C: IOContext>(
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

	/// Loads and parses the config field value.
	pub(crate) fn load_from_file<C: IOContext>(&self, context: &C) -> Option<T>
	where
		T: DeserializeOwned,
	{
		self.load_file(context).and_then(|json| self.extract_from_json_object(&json))
	}

	fn load_from_file_and_print(&self, context: &impl IOContext) -> Option<T>
	where
		T: DeserializeOwned + std::fmt::Display,
	{
		let value = self.load_from_file(context)?;
		context.eprint(&self.loaded_from_config_msg(&value));
		Some(value)
	}

	/// Updates the config field value in the file.
	pub(crate) fn save_to_file<C: IOContext>(&self, value: &T, context: &C)
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

	pub(crate) fn save_if_empty<C: IOContext>(&self, value: T, context: &C) -> T
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
	pub(crate) fn extract_from_json_object(&self, json: &serde_json::Value) -> Option<T>
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
	pub(crate) fn load_file<C: IOContext>(&self, context: &C) -> Option<serde_json::Value> {
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

	/// Print error message and exit.
	fn report_corrupted_file_and_quit(&self) -> ! {
		eprintln!(
			"Config file {} is broken. Delete it or fix manually and restart this wizard",
			self.config_file
		);
		exit(-1)
	}

	fn loaded_from_config_msg(&self, value: &T) -> String
	where
		T: std::fmt::Display,
	{
		format!("🛠️ Loaded {} from config ({}): {value}", self.name, self.config_file)
	}
}

#[derive(Clone, Debug)]
pub struct ServiceConfig {
	pub protocol: NetworkProtocol,
	pub hostname: String,
	pub port: u16,
	pub timeout_seconds: u64,
}

impl ServiceConfig {
	pub(crate) fn url(&self) -> String {
		format!("{}://{}:{}", self.protocol, self.hostname, self.port)
	}
}

pub(crate) trait SelectOptions {
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
/// A network protocol.
pub enum NetworkProtocol {
	#[serde(rename = "http")]
	/// Http protocol
	Http,
	#[serde(rename = "https")]
	/// Https protocol
	Https,
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
		match s.to_lowercase().as_str() {
			"http" => Ok(NetworkProtocol::Http),
			"https" => Ok(NetworkProtocol::Https),
			other => Err(format!("Invalid security protocol {other}, please provide http or http")),
		}
	}
}

impl SelectOptions for NetworkProtocol {
	fn select_options() -> Vec<String> {
		vec![NetworkProtocol::Http.to_string(), NetworkProtocol::Https.to_string()]
	}
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub(crate) struct MainChainAddresses {
	pub(crate) committee_candidates_address: String,
	/// unused
	pub(crate) d_parameter_policy_id: String,
	/// unused
	pub(crate) permissioned_candidates_policy_id: String,
	/// unused
	pub(crate) native_token: NativeTokenConfig,
}

#[derive(Deserialize, PartialEq, Clone, Debug)]
pub(crate) struct CardanoParameters {
	pub(crate) security_parameter: u64,
	pub(crate) active_slots_coeff: f64,
	pub(crate) first_epoch_number: u32,
	pub(crate) first_slot_number: u64,
	pub(crate) epoch_duration_millis: u64,
	pub(crate) first_epoch_timestamp_millis: u64,
	#[serde(default = "default_slot_duration_millis")]
	pub(crate) slot_duration_millis: u64,
}

fn default_slot_duration_millis() -> u64 {
	1000
}

impl CardanoParameters {
	pub(crate) fn save(&self, context: &impl IOContext) {
		CARDANO_SECURITY_PARAMETER.save_to_file(&self.security_parameter, context);
		CARDANO_ACTIVE_SLOTS_COEFF.save_to_file(&self.active_slots_coeff, context);
		CARDANO_FIRST_EPOCH_NUMBER.save_to_file(&self.first_epoch_number, context);
		CARDANO_FIRST_SLOT_NUMBER.save_to_file(&self.first_slot_number, context);
		CARDANO_EPOCH_DURATION_MILLIS.save_to_file(&self.epoch_duration_millis, context);
		CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS
			.save_to_file(&self.first_epoch_timestamp_millis, context);
		CARDANO_SLOT_DURATION_MILLIS.save_to_file(&self.slot_duration_millis, context);
	}
}

impl From<CardanoParameters> for sidechain_domain::mainchain_epoch::MainchainEpochConfig {
	fn from(value: CardanoParameters) -> Self {
		Self {
			first_epoch_timestamp_millis: Timestamp::from_unix_millis(
				value.first_epoch_timestamp_millis,
			),
			epoch_duration_millis: Duration::from_millis(value.epoch_duration_millis),
			first_epoch_number: value.first_epoch_number,
			first_slot_number: value.first_slot_number,
			slot_duration_millis: Duration::from_millis(value.slot_duration_millis),
		}
	}
}

#[derive(Deserialize, Serialize, Parser, Clone, Debug)]
pub(crate) struct SidechainParams {
	#[arg(long)]
	pub(crate) genesis_utxo: UtxoId,
}

impl Display for SidechainParams {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let json_string = serde_json::to_string_pretty(self).unwrap();
		write!(f, "{}", &json_string)
	}
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct AssetConfig {
	policy_id: String,
	asset_name: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct NativeTokenConfig {
	pub(crate) asset: AssetConfig,
	pub(crate) illiquid_supply_address: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct GovernanceAuthoritiesKeyHashes(pub(crate) Vec<MainchainKeyHash>);

impl FromStr for GovernanceAuthoritiesKeyHashes {
	type Err = String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mut key_hashes = vec![];
		for x in s.split_whitespace() {
			let trimmed = x.trim();
			let key_hash = MainchainKeyHash::decode_hex(trimmed).map_err(|e| {
				format!("'{}' is not a valid Cardano Key Hash, reason: {}", trimmed, e)
			})?;
			key_hashes.push(key_hash)
		}
		Ok(GovernanceAuthoritiesKeyHashes(key_hashes))
	}
}

impl Display for GovernanceAuthoritiesKeyHashes {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let mut it = self.0.iter();
		// Intersperse with a single space
		if let Some(key_hash) = it.next() {
			f.write_str(&key_hash.to_hex_string())?;
			for key_hash in it {
				f.write_char(' ')?;
				f.write_str(&key_hash.to_hex_string())?;
			}
		}
		Ok(())
	}
}

#[derive(Deserialize)]
pub(crate) struct ChainConfig {
	pub(crate) cardano: CardanoParameters,
	pub(crate) chain_parameters: SidechainParams,
	pub(crate) cardano_addresses: MainChainAddresses,
}

pub(crate) const KEYS_FILE_PATH: &str = "partner-chains-public-keys.json";
pub(crate) const CHAIN_CONFIG_FILE_PATH: &str = "pc-chain-config.json";
pub(crate) const RESOURCES_CONFIG_FILE_PATH: &str = "pc-resources-config.json";
pub(crate) const CHAIN_SPEC_PATH: &str = "chain-spec.json";

pub(crate) fn load_chain_config(context: &impl IOContext) -> anyhow::Result<ChainConfig> {
	if let Some(chain_config_file) = context.read_file(CHAIN_CONFIG_FILE_PATH) {
		serde_json::from_str::<ChainConfig>(&chain_config_file)
			.map_err(|err| anyhow::anyhow!(format!("⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} is invalid: {err}. Run prepare-configuration wizard or fix errors manually.")))
	} else {
		Err(anyhow::anyhow!(format!(
			"⚠️ Chain config file {CHAIN_CONFIG_FILE_PATH} does not exists. Run prepare-configuration wizard first."
		)))
	}
}

pub(crate) mod config_fields {
	use super::*;
	use sidechain_domain::UtxoId;

	pub(crate) const NATIVE_TOKEN_POLICY: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "native_token", "asset", "policy_id"],
			name: "native token policy ID",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const NATIVE_TOKEN_ASSET_NAME: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "native_token", "asset", "asset_name"],
			name: "native token asset name in hex",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const ILLIQUID_SUPPLY_ADDRESS: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "native_token", "illiquid_supply_address"],
			name: "native token illiquid token supply address",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const SUBSTRATE_NODE_DATA_BASE_PATH: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["substrate_node_base_path"],
			name: "node base path",
			default: Some("./data"),
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_PAYMENT_VERIFICATION_KEY_FILE: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["cardano_payment_verification_key_file"],
			name: "path to the payment verification file",
			default: Some("payment.vkey"),
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_PAYMENT_SIGNING_KEY_FILE: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["cardano_payment_signing_key_file"],
			name: "path to the payment signing key file",
			default: Some("payment.skey"),
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_COLD_VERIFICATION_KEY_FILE: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["cardano_cold_verification_key_file"],
			name: "path to the cold verification key file",
			default: Some("cold.vkey"),
			_marker: PhantomData,
		};

	pub(crate) const GENESIS_UTXO: ConfigFieldDefinition<'static, UtxoId> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["chain_parameters", "genesis_utxo"],
		name: "genesis utxo",
		default: None,
		_marker: PhantomData,
	};

	pub(crate) const BOOTNODES: ConfigFieldDefinition<'static, Vec<String>> =
		ConfigFieldDefinition {
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

	pub(crate) const POSTGRES_CONNECTION_STRING: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["db_sync_postgres_connection_string"],
			name: "DB-Sync Postgres connection string",
			default: Some("postgresql://postgres-user:postgres-password@localhost:5432/cexplorer"),
			_marker: PhantomData,
		};

	pub(crate) const OGMIOS_PROTOCOL: ConfigFieldDefinition<'static, NetworkProtocol> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["ogmios", "protocol"],
			name: "Ogmios protocol (http/https)",
			default: Some("http"),
			_marker: PhantomData,
		};

	pub(crate) const OGMIOS_HOSTNAME: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["ogmios", "hostname"],
			name: "Ogmios hostname",
			default: Some("localhost"),
			_marker: PhantomData,
		};

	pub(crate) const OGMIOS_PORT: ConfigFieldDefinition<'static, u16> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["ogmios", "port"],
		name: "Ogmios port",
		default: Some("1337"),
		_marker: PhantomData,
	};

	pub(crate) const OGMIOS_REQUEST_TIMEOUT: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: RESOURCES_CONFIG_FILE_PATH,
			path: &["ogmios", "request_timeout"],
			name: "Ogmios request timeout [seconds]",
			default: Some("180"),
			_marker: PhantomData,
		};

	pub(crate) const COMMITTEE_CANDIDATES_ADDRESS: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "committee_candidates_address"],
			name: "Committee candidates address",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const D_PARAMETER_POLICY_ID: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "d_parameter_policy_id"],
			name: "D parameter policy id",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const PERMISSIONED_CANDIDATES_POLICY_ID: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "permissioned_candidates_policy_id"],
			name: "permissioned candidates policy id",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const GOVERNED_MAP_VALIDATOR_ADDRESS: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "governed_map", "validator_address"],
			name: "Governed Map Validator Address",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const GOVERNED_MAP_POLICY_ID: ConfigFieldDefinition<'static, String> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano_addresses", "governed_map", "policy_id"],
			name: "Governed Map Policy Id",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_SECURITY_PARAMETER: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "security_parameter"],
			name: "cardano security parameter",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_ACTIVE_SLOTS_COEFF: ConfigFieldDefinition<'static, f64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "active_slots_coeff"],
			name: "cardano active slot coefficient",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_FIRST_EPOCH_NUMBER: ConfigFieldDefinition<'static, u32> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "first_epoch_number"],
			name: "cardano first epoch number in shelley era",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_FIRST_SLOT_NUMBER: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "first_slot_number"],
			name: "cardano first slot number in shelley era",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_EPOCH_DURATION_MILLIS: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "epoch_duration_millis"],
			name: "cardano epoch duration in millis",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "first_epoch_timestamp_millis"],
			name: "cardano first shelley epoch timestamp in millis",
			default: None,
			_marker: PhantomData,
		};

	pub(crate) const CARDANO_SLOT_DURATION_MILLIS: ConfigFieldDefinition<'static, u64> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["cardano", "slot_duration_millis"],
			name: "cardano slot duration in millis",
			default: Some("1000"),
			_marker: PhantomData,
		};

	pub(crate) const NODE_P2P_PORT: ConfigFieldDefinition<'static, u16> = ConfigFieldDefinition {
		config_file: RESOURCES_CONFIG_FILE_PATH,
		path: &["node_p2p_port"],
		name: "substrate-node p2p protocol TCP port",
		default: Some("30333"),
		_marker: PhantomData,
	};

	pub(crate) const INITIAL_GOVERNANCE_AUTHORITIES: ConfigFieldDefinition<
		'static,
		GovernanceAuthoritiesKeyHashes,
	> = ConfigFieldDefinition {
		config_file: CHAIN_CONFIG_FILE_PATH,
		path: &["initial_governance", "authorities"],
		name: "space separated keys hashes of the initial Multisig Governance Authorities",
		default: Some("[]"),
		_marker: PhantomData,
	};

	pub(crate) const INITIAL_GOVERNANCE_THRESHOLD: ConfigFieldDefinition<'static, u8> =
		ConfigFieldDefinition {
			config_file: CHAIN_CONFIG_FILE_PATH,
			path: &["initial_governance", "threshold"],
			name: "Initial Multisig Governance Threshold",
			default: Some("1"),
			_marker: PhantomData,
		};
}
