use crate::config::config_fields::{NODE_EXECUTABLE, NODE_EXECUTABLE_DEFAULT};
use crate::config::ConfigFieldDefinition;
use crate::io::IOContext;
use crate::permissioned_candidates::{ParsedPermissionedCandidatesKeys, PermissionedCandidateKeys};
use crate::{config::config_fields, CmdRun};
use anyhow::{anyhow, Context};
use serde::de::DeserializeOwned;
use serde_json::Value as JValue;
use sidechain_domain::{MainchainAddressHash, UtxoId};

#[cfg(test)]
mod tests;

#[derive(Debug, clap::Parser)]
pub struct CreateChainSpecCmd;

const SESSION_INITIAL_VALIDATORS_PATH: &str =
	"/genesis/runtimeGenesis/config/session/initialValidators";
const SESSION_VALIDATOR_MANAGEMENT_INITIAL_AUTHORITIES_PATH: &str =
	"/genesis/runtimeGenesis/config/sessionCommitteeManagement/initialAuthorities";

impl CmdRun for CreateChainSpecCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let config = CreateChainSpecConfig::load(context)?;
		context.print("This wizard will create a chain spec JSON file according to the provided configuration, using WASM runtime code from the compiled node binary.");
		Self::print_config(context, &config);
		if context.prompt_yes_no("Do you want to continue?", true) {
			Self::run_build_spec_command(context, &config)?;
			Self::update_chain_spec_authorities(context, &config)?;
			context.print("chain-spec.json file has been created.");
			context.print(
				"If you are the governance authority, you can distribute it to the validators.",
			);
			context.print("Run 'setup-main-chain-state' command to set D-parameter and permissioned candidates on Cardano.");
			Ok(())
		} else {
			context.print("Aborted.");
			Ok(())
		}
	}
}

impl CreateChainSpecCmd {
	fn print_config<C: IOContext>(context: &C, config: &CreateChainSpecConfig) {
		context.print("Chain parameters:");
		context.print(format!("- Chain ID: {}", config.chain_id).as_str());
		context.print(
			format!("- Governance authority: {}", config.governance_authority.to_hex_string())
				.as_str(),
		);
		context.print(
			"Legacy parameters (keep defaults as long as you are not sure to do otherwise):",
		);
		context.print(format!("- Threshold numerator: {}", config.threshold_numerator).as_str());
		context
			.print(format!("- Threshold denominator: {}", config.threshold_denominator).as_str());
		context.print(
			format!("- Genesis committee hash UTXO: {}", config.genesis_committee_utxo).as_str(),
		);
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
		use colored::Colorize;
		if config.initial_permissioned_candidates_raw.is_empty() {
			context.print("WARNING: The list of initial permissioned candidates is empty. Generated chain spec will not allow the chain to start.".red().to_string().as_str());
			let update_msg = format!("Update 'initial_permissioned_candidates' field of {} file with keys of initial committee.", config_fields::INITIAL_PERMISSIONED_CANDIDATES.config_file);
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
		let node_executable =
			NODE_EXECUTABLE.save_if_empty(NODE_EXECUTABLE_DEFAULT.to_string(), context);
		context.set_env_var("CHAIN_ID", &config.chain_id.to_string());
		context.set_env_var("GOVERNANCE_AUTHORITY", &config.governance_authority.to_string());
		context.set_env_var("THRESHOLD_NUMERATOR", &config.threshold_numerator.to_string());
		context.set_env_var("THRESHOLD_DENOMINATOR", &config.threshold_denominator.to_string());
		context.set_env_var("GENESIS_COMMITTEE_UTXO", &config.genesis_committee_utxo.to_string());
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

	fn update_chain_spec_authorities<C: IOContext>(
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
			.map(|c| serde_json::to_value((c.account_id_32(), c.session_keys())))
			.collect::<Result<Vec<serde_json::Value>, _>>()?;
		let initial_validators = serde_json::Value::Array(initial_validators);
		Self::update_field(&mut chain_spec, SESSION_INITIAL_VALIDATORS_PATH, initial_validators)?;

		let initial_authorities = config
			.initial_permissioned_candidates_parsed
			.iter()
			.map(|c| serde_json::to_value((c.sidechain, c.session_keys())))
			.collect::<Result<Vec<serde_json::Value>, _>>()?;
		let initial_authorities = serde_json::Value::Array(initial_authorities);
		Self::update_field(
			&mut chain_spec,
			SESSION_VALIDATOR_MANAGEMENT_INITIAL_AUTHORITIES_PATH,
			initial_authorities,
		)?;
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
	chain_id: u16,
	governance_authority: MainchainAddressHash,
	threshold_numerator: u64,
	threshold_denominator: u64,
	genesis_committee_utxo: UtxoId,
	initial_permissioned_candidates_raw: Vec<PermissionedCandidateKeys>,
	initial_permissioned_candidates_parsed: Vec<ParsedPermissionedCandidatesKeys>,
	committee_candidate_address: String,
	d_parameter_policy_id: String,
	permissioned_candidates_policy_id: String,
	native_token_policy: String,
	native_token_asset_name: String,
	illiquid_supply_address: String,
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
			chain_id: load_config_field(c, &config_fields::CHAIN_ID)?,
			governance_authority: load_config_field(c, &config_fields::GOVERNANCE_AUTHORITY)?,
			threshold_numerator: load_config_field(c, &config_fields::THRESHOLD_NUMERATOR)?,
			threshold_denominator: load_config_field(c, &config_fields::THRESHOLD_DENOMINATOR)?,
			genesis_committee_utxo: load_config_field(c, &config_fields::GENESIS_COMMITTEE_UTXO)?,
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
