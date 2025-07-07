use crate::config::ConfigFieldDefinition;
use crate::io::IOContext;
use crate::permissioned_candidates::{ParsedPermissionedCandidatesKeys, PermissionedCandidateKeys};
use crate::runtime_bindings::PartnerChainRuntime;
use crate::{CmdRun, config::config_fields};
use anyhow::anyhow;
use sidechain_domain::{AssetName, MainchainAddress, PolicyId, UtxoId};
use sp_runtime::DeserializeOwned;
use std::marker::PhantomData;

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Default, clap::Parser)]
pub struct CreateChainSpecCmd<T: PartnerChainRuntime> {
	#[clap(skip)]
	_phantom: PhantomData<T>,
}

impl<T: PartnerChainRuntime> CmdRun for CreateChainSpecCmd<T> {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let config = CreateChainSpecConfig::load(context)?;
		context.print("This wizard will create a chain spec JSON file according to the provided configuration, using WASM runtime code from the compiled node binary.");
		Self::print_config(context, &config);
		if context.prompt_yes_no("Do you want to continue?", true) {
			let content = T::create_chain_spec(&config);
			context.write_file("chain-spec.json", &serde_json::to_string_pretty(&content)?);
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
		context.print(
			format!("- d_parameter_policy_id: {}", config.d_parameter_policy_id.to_hex_string())
				.as_str(),
		);
		context.print(
			format!(
				"- permissioned_candidates_policy_id: {}",
				config.permissioned_candidates_policy_id.to_hex_string()
			)
			.as_str(),
		);
		context.print("Native Token Management Configuration (unused if empty):");
		context.print(&format!("- asset name: {}", config.native_token_asset_name.to_hex_string()));
		context
			.print(&format!("- asset policy ID: {}", config.native_token_policy.to_hex_string()));
		context.print(&format!("- illiquid supply address: {}", config.illiquid_supply_address));
		context.print("Governed Map Configuration:");
		context.print(&format!(
			"- validator address: {}",
			config.governed_map_validator_address.clone().unwrap_or_default()
		));
		context.print(&format!(
			"- asset policy ID: {}",
			config.governed_map_asset_policy_id.clone().unwrap_or_default().to_hex_string()
		));
		use colored::Colorize;
		if config.initial_permissioned_candidates_parsed.is_empty() {
			context.print("WARNING: The list of initial permissioned candidates is empty. Generated chain spec will not allow the chain to start.".red().to_string().as_str());
			let update_msg = format!(
				"Update 'initial_permissioned_candidates' field of {} file with keys of initial committee.",
				context
					.config_file_path(config_fields::INITIAL_PERMISSIONED_CANDIDATES.config_file)
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
}

#[allow(missing_docs)]
#[derive(Debug)]
/// Configuration that contains all Partner Chain specific data required to create the chain spec
pub struct CreateChainSpecConfig {
	pub genesis_utxo: UtxoId,
	pub initial_permissioned_candidates_raw: Vec<PermissionedCandidateKeys>,
	pub initial_permissioned_candidates_parsed: Vec<ParsedPermissionedCandidatesKeys>,
	pub committee_candidate_address: MainchainAddress,
	pub d_parameter_policy_id: PolicyId,
	pub permissioned_candidates_policy_id: PolicyId,
	pub native_token_policy: PolicyId,
	pub native_token_asset_name: AssetName,
	pub illiquid_supply_address: MainchainAddress,
	pub governed_map_validator_address: Option<MainchainAddress>,
	pub governed_map_asset_policy_id: Option<PolicyId>,
}

impl CreateChainSpecConfig {
	pub(crate) fn load<C: IOContext>(c: &C) -> Result<Self, anyhow::Error> {
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

	/// Returns [pallet_sidechain::GenesisConfig] derived from the config
	pub fn pallet_sidechain_config<T: pallet_sidechain::Config>(
		&self,
		slots_per_epoch: sidechain_slots::SlotsPerEpoch,
	) -> pallet_sidechain::GenesisConfig<T> {
		pallet_sidechain::GenesisConfig {
			genesis_utxo: self.genesis_utxo,
			slots_per_epoch,
			_config: PhantomData,
		}
	}

	/// Returns [pallet_session::GenesisConfig] derived from the config, using initial permissioned candidates
	/// as initial validators
	pub fn pallet_partner_chains_session_config<
		T: pallet_partner_chains_session::Config,
		F: Fn(&ParsedPermissionedCandidatesKeys) -> (T::ValidatorId, T::Keys),
	>(
		&self,
		f: F,
	) -> pallet_partner_chains_session::GenesisConfig<T> {
		pallet_partner_chains_session::GenesisConfig {
			initial_validators: self
				.initial_permissioned_candidates_parsed
				.iter()
				.map(|c| f(c))
				.collect::<Vec<_>>(),
		}
	}

	/// Returns [pallet_session_validator_management::GenesisConfig] derived from the config using initial permissioned candidates
	/// as initial authorities
	pub fn pallet_session_validator_management_config<
		T: pallet_session_validator_management::Config,
		F: Fn(&ParsedPermissionedCandidatesKeys) -> T::CommitteeMember,
	>(
		&self,
		f: F,
	) -> pallet_session_validator_management::GenesisConfig<T> {
		pallet_session_validator_management::GenesisConfig {
			initial_authorities: self
				.initial_permissioned_candidates_parsed
				.iter()
				.map(|c| f(c))
				.collect::<Vec<_>>(),
			main_chain_scripts: sp_session_validator_management::MainChainScripts {
				committee_candidate_address: self.committee_candidate_address.clone(),
				d_parameter_policy_id: self.d_parameter_policy_id.clone(),
				permissioned_candidates_policy_id: self.permissioned_candidates_policy_id.clone(),
			},
		}
	}

	/// Returns [pallet_native_token_management::GenesisConfig] derived from the config
	pub fn native_token_management_config<T: pallet_native_token_management::Config>(
		&self,
	) -> pallet_native_token_management::GenesisConfig<T> {
		pallet_native_token_management::GenesisConfig {
			main_chain_scripts: Some(sp_native_token_management::MainChainScripts {
				native_token_policy_id: self.native_token_policy.clone(),
				native_token_asset_name: self.native_token_asset_name.clone(),
				illiquid_supply_validator_address: self.illiquid_supply_address.clone(),
			}),
			_marker: PhantomData,
		}
	}

	/// Returns [pallet_governed_map::GenesisConfig] derived from the config
	pub fn governed_map_config<T: pallet_governed_map::Config>(
		&self,
	) -> pallet_governed_map::GenesisConfig<T> {
		pallet_governed_map::GenesisConfig {
			main_chain_scripts: self.governed_map_validator_address.as_ref().and_then(|addr| {
				self.governed_map_asset_policy_id.as_ref().map(|policy| {
					sp_governed_map::MainChainScriptsV1 {
						validator_address: addr.clone(),
						asset_policy_id: policy.clone(),
					}
				})
			}),
			_marker: PhantomData,
		}
	}
}

fn load_config_field<C: IOContext, T: DeserializeOwned>(
	context: &C,
	field: &ConfigFieldDefinition<T>,
) -> Result<T, anyhow::Error> {
	field.load_from_file(context).ok_or_else(|| {
		context.eprint(format!("The '{}' configuration file is missing or invalid.\nIf you are the governance authority, please make sure you have run the `prepare-configuration` command to generate the chain configuration file.\nIf you are a validator, you can obtain the chain configuration file from the governance authority.", context.config_file_path(field.config_file)).as_str());
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
