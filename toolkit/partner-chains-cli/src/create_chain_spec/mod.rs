use crate::config::ConfigFieldDefinition;
use crate::io::IOContext;
use crate::permissioned_candidates::{ParsedPermissionedCandidatesKeys, PermissionedCandidateKeys};
use crate::runtime_bindings::PartnerChainRuntime;
use crate::{CmdRun, config::config_fields};
use anyhow::{Context, anyhow};
use serde_json::Value as JValue;
use sidechain_domain::UtxoId;
use sp_runtime::DeserializeOwned;
use std::marker::PhantomData;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Default, clap::Parser)]
pub struct CreateChainSpecCmd<T: PartnerChainRuntime> {
	#[clap(skip)]
	_phantom: PhantomData<T>,
}

const SESSION_INITIAL_VALIDATORS_PATH: &str =
	"/genesis/runtimeGenesis/config/session/initialValidators";
const SESSION_VALIDATOR_MANAGEMENT_INITIAL_AUTHORITIES_PATH: &str =
	"/genesis/runtimeGenesis/config/sessionCommitteeManagement/initialAuthorities";
const GOVERNED_MAP_VALIDATOR_ADDRESS_PATH: &str =
	"/genesis/runtimeGenesis/config/governedMap/mainChainScripts/validator_address";
const GOVERNED_MAP_ASSET_POLICY_ID_PATH: &str =
	"/genesis/runtimeGenesis/config/governedMap/mainChainScripts/asset_policy_id";

impl<T: PartnerChainRuntime> CmdRun for CreateChainSpecCmd<T> {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let config = CreateChainSpecConfig::load(context)?;
		context.print("This wizard will create a chain spec JSON file according to the provided configuration, using WASM runtime code from the compiled node binary.");
		Self::print_config(context, &config);
		if context.prompt_yes_no("Do you want to continue?", true) {
			Self::run_build_spec_command(context, &config)?;
			Self::update_chain_spec_field_not_filled_by_the_node(context, &config)?;
			context.print("chain-spec.json file has been created.");
			context.print(
				"If you are the governance authority, you can distribute it to the validators.",
			);
			Ok(())
		} else {
			context.print("Aborted.");
			Ok(())
		}
	}
}

impl<T: PartnerChainRuntime> CreateChainSpecCmd<T> {
	fn print_config<C: IOContext>(context: &C, config: &CreateChainSpecConfig) {
		context.print("Chain parameters:");
		context.print(format!("- Genesis UTXO: {}", config.genesis_utxo).as_str());
		context.print("SessionValidatorManagement Main Chain Configuration:");
		context.print(
			format!("- committee_candidate_address: {}", config.committee_candidate_address)
				.as_str(),
		);
		context
			.print(format!("- d_parameter_policy_id: {}", config.d_parameter_policy_id).as_str());
		context.print(
			format!(
				"- permissioned_candidates_policy_id: {}",
				config.permissioned_candidates_policy_id
			)
			.as_str(),
		);
		context.print("Native Token Management Configuration (unused if empty):");
		context.print(&format!("- asset name: {}", config.native_token_asset_name));
		context.print(&format!("- asset policy ID: {}", config.native_token_policy));
		context.print(&format!("- illiquid supply address: {}", config.illiquid_supply_address));
		context.print("Governed Map Configuration:");
		context.print(&format!(
			"- validator address: {}",
			config.governed_map_validator_address.clone().unwrap_or_default()
		));
		context.print(&format!(
			"- asset policy ID: {}",
			config.governed_map_asset_policy_id.clone().unwrap_or_default()
		));
		use colored::Colorize;
		if config.initial_permissioned_candidates_raw.is_empty() {
			context.print("WARNING: The list of initial permissioned candidates is empty. Generated chain spec will not allow the chain to start.".red().to_string().as_str());
			let update_msg = format!(
				"Update 'initial_permissioned_candidates' field of {} file with keys of initial committee.",
				config_fields::INITIAL_PERMISSIONED_CANDIDATES.config_file
			);
			context.print(update_msg.red().to_string().as_str());
			context.print(INITIAL_PERMISSIONED_CANDIDATES_EXAMPLE.yellow().to_string().as_str());
		} else {
			context.print("Initial permissioned candidates:");
			for candidate in config.initial_permissioned_candidates_raw.iter() {
				context.print(format!("- {}", candidate).as_str());
			}
		}
	}

	fn run_build_spec_command<C: IOContext>(
		context: &C,
		config: &CreateChainSpecConfig,
	) -> anyhow::Result<String> {
		let node_executable = context.current_executable()?;
		context.set_env_var("GENESIS_UTXO", &config.genesis_utxo.to_string());
		context.set_env_var(
			"COMMITTEE_CANDIDATE_ADDRESS",
			&config.committee_candidate_address.to_string(),
		);
		context.set_env_var("D_PARAMETER_POLICY_ID", &config.d_parameter_policy_id.to_string());
		context.set_env_var(
			"PERMISSIONED_CANDIDATES_POLICY_ID",
			&config.permissioned_candidates_policy_id.to_string(),
		);
		context.set_env_var("NATIVE_TOKEN_POLICY_ID", &config.native_token_policy);
		context.set_env_var("NATIVE_TOKEN_ASSET_NAME", &config.native_token_asset_name);
		context.set_env_var("ILLIQUID_SUPPLY_VALIDATOR_ADDRESS", &config.illiquid_supply_address);
		context.run_command(
			format!("{node_executable} build-spec --disable-default-bootnode > chain-spec.json")
				.to_string()
				.as_str(),
		)
	}

	fn update_chain_spec_field_not_filled_by_the_node<C: IOContext>(
		context: &C,
		config: &CreateChainSpecConfig,
	) -> anyhow::Result<()> {
		let json = context
			.read_file("chain-spec.json")
			.context("Could not read chain-spec.json file. File is expected to exists.")?;
		let mut chain_spec: serde_json::Value = serde_json::from_str(&json)?;

		let initial_validators = config
			.initial_permissioned_candidates_parsed
			.iter()
			.map(|c| {
				serde_json::to_value((c.account_id_32(), c.session_keys::<T::AuthorityKeys>()))
			})
			.collect::<Result<Vec<serde_json::Value>, _>>()?;
		let initial_validators = serde_json::Value::Array(initial_validators);
		Self::update_field(&mut chain_spec, SESSION_INITIAL_VALIDATORS_PATH, initial_validators)?;

		let initial_authorities = config
			.initial_permissioned_candidates_parsed
			.iter()
			.map(|c| -> anyhow::Result<serde_json::Value> {
				let initial_member =
					T::initial_member(c.sidechain.into(), c.session_keys::<T::AuthorityKeys>());
				Ok(serde_json::to_value(initial_member)?)
			})
			.collect::<Result<Vec<serde_json::Value>, _>>()?;
		let initial_authorities = serde_json::Value::Array(initial_authorities);
		Self::update_field(
			&mut chain_spec,
			SESSION_VALIDATOR_MANAGEMENT_INITIAL_AUTHORITIES_PATH,
			initial_authorities,
		)?;
		match config.governed_map_validator_address.clone() {
			Some(address) => Self::update_field(
				&mut chain_spec,
				GOVERNED_MAP_VALIDATOR_ADDRESS_PATH,
				serde_json::Value::String(format!("0x{}", hex::encode(address.as_bytes()))),
			)?,
			None => (),
		}
		match config.governed_map_asset_policy_id.clone() {
			Some(policy_id) => Self::update_field(
				&mut chain_spec,
				GOVERNED_MAP_ASSET_POLICY_ID_PATH,
				serde_json::Value::String(format!("0x{policy_id}")),
			)?,
			None => (),
		}
		context.write_file("chain-spec.json", serde_json::to_string_pretty(&chain_spec)?.as_str());
		Ok(())
	}

	fn update_field(
		chain_spec: &mut JValue,
		field_name: &str,
		value: JValue,
	) -> Result<(), anyhow::Error> {
		if let Some(field) = chain_spec.pointer_mut(field_name) {
			*field = value;
			Ok(())
		} else {
			Err(anyhow!(
				"Internal error: Could not find {field_name} in chain spec file! Possibly this wizard does not support the current chain spec version."
			))
		}
	}
}

#[derive(Debug)]
struct CreateChainSpecConfig {
	genesis_utxo: UtxoId,
	initial_permissioned_candidates_raw: Vec<PermissionedCandidateKeys>,
	initial_permissioned_candidates_parsed: Vec<ParsedPermissionedCandidatesKeys>,
	committee_candidate_address: String,
	d_parameter_policy_id: String,
	permissioned_candidates_policy_id: String,
	native_token_policy: String,
	native_token_asset_name: String,
	illiquid_supply_address: String,
	governed_map_validator_address: Option<String>,
	governed_map_asset_policy_id: Option<String>,
}

impl CreateChainSpecConfig {
	pub fn load<C: IOContext>(c: &C) -> Result<Self, anyhow::Error> {
		let initial_permissioned_candidates_raw =
			load_config_field(c, &config_fields::INITIAL_PERMISSIONED_CANDIDATES)?;
		let initial_permissioned_candidates_parsed: Vec<ParsedPermissionedCandidatesKeys> =
			initial_permissioned_candidates_raw
				.iter()
				.map(TryFrom::try_from)
				.collect::<Result<Vec<ParsedPermissionedCandidatesKeys>, anyhow::Error>>()?;
		Ok(Self {
			genesis_utxo: load_config_field(c, &config_fields::GENESIS_UTXO)?,
			initial_permissioned_candidates_raw,
			initial_permissioned_candidates_parsed,
			committee_candidate_address: load_config_field(
				c,
				&config_fields::COMMITTEE_CANDIDATES_ADDRESS,
			)?,
			d_parameter_policy_id: load_config_field(c, &config_fields::D_PARAMETER_POLICY_ID)?,
			permissioned_candidates_policy_id: load_config_field(
				c,
				&config_fields::PERMISSIONED_CANDIDATES_POLICY_ID,
			)?,
			native_token_policy: load_config_field(c, &config_fields::NATIVE_TOKEN_POLICY)?,
			native_token_asset_name: load_config_field(c, &config_fields::NATIVE_TOKEN_ASSET_NAME)?,
			illiquid_supply_address: load_config_field(c, &config_fields::ILLIQUID_SUPPLY_ADDRESS)?,
			governed_map_validator_address: config_fields::GOVERNED_MAP_VALIDATOR_ADDRESS
				.load_from_file(c),
			governed_map_asset_policy_id: config_fields::GOVERNED_MAP_POLICY_ID.load_from_file(c),
		})
	}
}

fn load_config_field<C: IOContext, T: DeserializeOwned>(
	context: &C,
	field: &ConfigFieldDefinition<T>,
) -> Result<T, anyhow::Error> {
	field.load_from_file(context).ok_or_else(|| {
		context.eprint(format!("The '{}' configuration file is missing or invalid.\nIf you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.\nIf you are a validator, you can obtain the chain configuration file from the governance authority.", field.config_file).as_str());
		anyhow!("failed to read '{}'", field.path.join("."))
	})
}

pub const INITIAL_PERMISSIONED_CANDIDATES_EXAMPLE: &str = r#"Example of 'initial_permissioned_candidates' field with 2 permissioned candidates:
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
]"#;
