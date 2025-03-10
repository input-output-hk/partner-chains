use crate::config::config_fields::{CARDANO_PAYMENT_SIGNING_KEY_FILE, POSTGRES_CONNECTION_STRING};
use crate::config::{
	config_fields, ChainConfig, ConfigFieldDefinition, ServiceConfig, CHAIN_CONFIG_FILE_PATH,
};
use crate::io::IOContext;
use crate::ogmios::config::prompt_ogmios_configuration;
use crate::permissioned_candidates::{ParsedPermissionedCandidatesKeys, PermissionedCandidateKeys};
use crate::{cardano_key, CmdRun};
use anyhow::anyhow;
use anyhow::Context;
use partner_chains_cardano_offchain::d_param::UpsertDParam;
use partner_chains_cardano_offchain::permissioned_candidates::UpsertPermissionedCandidates;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sidechain_domain::mainchain_epoch::MainchainEpochDerivation;
use sidechain_domain::{McEpochNumber, UtxoId};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, clap::Parser)]
pub struct SetupMainChainStateCmd;

/// Formats of the output of the `ariadne-parameters` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AriadneParametersOutput {
	pub d_parameter: DParameter,
	pub permissioned_candidates: Vec<PermissionedCandidateData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DParameter {
	pub num_permissioned_candidates: u64,
	pub num_registered_candidates: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionedCandidateData {
	pub sidechain_public_key: String,
	pub aura_public_key: String,
	pub grandpa_public_key: String,
	pub is_valid: bool,
}

impl TryFrom<PermissionedCandidateData> for ParsedPermissionedCandidatesKeys {
	type Error = anyhow::Error;

	fn try_from(value: PermissionedCandidateData) -> Result<Self, Self::Error> {
		let keys = PermissionedCandidateKeys {
			sidechain_pub_key: value.sidechain_public_key,
			aura_pub_key: value.aura_public_key,
			grandpa_pub_key: value.grandpa_public_key,
		};
		TryFrom::try_from(&keys)
	}
}

struct AriadneParameters {
	d_parameter: DParameter,
	permissioned_candidates: SortedPermissionedCandidates,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct SortedPermissionedCandidates(Vec<ParsedPermissionedCandidatesKeys>);

impl SortedPermissionedCandidates {
	pub fn new(mut keys: Vec<ParsedPermissionedCandidatesKeys>) -> Self {
		keys.sort();
		Self(keys)
	}

	pub fn to_candidate_data(&self) -> Vec<sidechain_domain::PermissionedCandidateData> {
		self.0
			.iter()
			.map(|c| sidechain_domain::PermissionedCandidateData {
				sidechain_public_key: c.sidechain.into(),
				aura_public_key: c.aura.into(),
				grandpa_public_key: c.grandpa.into(),
			})
			.collect()
	}
}

impl CmdRun for SetupMainChainStateCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let chain_config = crate::config::load_chain_config(context)?;
		context.print(
			"This wizard will set or update D-Parameter and Permissioned Candidates on the main chain. Setting either of these costs ADA!",
		);
		let config_initial_authorities =
			initial_permissioned_candidates_from_chain_config(context)?;

		if let Some(ariadne_parameters) = get_ariadne_parameters(context, &chain_config)? {
			let ogmios_config: Option<ServiceConfig> = if ariadne_parameters.permissioned_candidates
				== config_initial_authorities
			{
				context.print(&format!("Permissioned candidates in the {} file match the most recent on-chain initial permissioned candidates.", CHAIN_CONFIG_FILE_PATH));
				None
			} else {
				print_on_chain_and_config_permissioned_candidates(
					context,
					&ariadne_parameters.permissioned_candidates,
					&config_initial_authorities,
				);
				set_candidates_on_main_chain(
					context,
					config_initial_authorities,
					chain_config.chain_parameters.genesis_utxo,
				)?
			};
			context.print(&format!(
				"D-Parameter on the main chain is: (P={}, R={})",
				ariadne_parameters.d_parameter.num_permissioned_candidates,
				ariadne_parameters.d_parameter.num_registered_candidates
			));
			set_d_parameter_on_main_chain(
				context,
				ogmios_config,
				ariadne_parameters.d_parameter,
				chain_config.chain_parameters.genesis_utxo,
			)?;
		} else {
			let ogmios_config: Option<ServiceConfig> = set_candidates_on_main_chain(
				context,
				config_initial_authorities,
				chain_config.chain_parameters.genesis_utxo,
			)?;
			let default_d_parameter =
				DParameter { num_permissioned_candidates: 0, num_registered_candidates: 0 };
			set_d_parameter_on_main_chain(
				context,
				ogmios_config,
				default_d_parameter,
				chain_config.chain_parameters.genesis_utxo,
			)?;
		}
		context.print("Done. Main chain state is set. Please remember that any changes can be observed immediately, but from the Partner Chain point of view they will be effective in two main chain epochs.");
		Ok(())
	}
}

fn initial_permissioned_candidates_from_chain_config<C: IOContext>(
	context: &C,
) -> anyhow::Result<SortedPermissionedCandidates> {
	// Requirements state "read from 'chain config' (or chain-spec).
	// It's easier to read from config than from chain-spec, because parsing is already present.
	let candidates: Vec<PermissionedCandidateKeys> =
		load_chain_config_field(context, &config_fields::INITIAL_PERMISSIONED_CANDIDATES)?;
	let candidates = candidates
		.iter()
		.map(ParsedPermissionedCandidatesKeys::try_from)
		.collect::<Result<Vec<_>, _>>()?;
	Ok(SortedPermissionedCandidates::new(candidates))
}

fn get_ariadne_parameters<C: IOContext>(
	context: &C,
	chain_config: &ChainConfig,
) -> anyhow::Result<Option<AriadneParameters>> {
	context.print("Will read the current D-Parameter and Permissioned Candidates from the main chain, using 'partner-chains-node ariadne-parameters' command.");
	let postgres_connection_string =
		POSTGRES_CONNECTION_STRING.prompt_with_default_from_file_and_save(context);
	crate::main_chain_follower::set_main_chain_follower_env(
		context,
		&chain_config.cardano,
		&postgres_connection_string,
	);
	let executable = context.current_executable()?;
	// Call for state that will be effective in two main chain epochs from now.
	let epoch = get_current_mainchain_epoch(context, chain_config)?.0 + 2;
	let temp_dir = context.new_tmp_dir();
	let temp_dir_path = temp_dir
		.into_os_string()
		.into_string()
		.expect("PathBuf is a valid UTF-8 String");
	let output = context
		.run_command(&format!("{executable} ariadne-parameters --base-path {temp_dir_path} --chain chain-spec.json --mc-epoch-number {epoch}"))?;
	context.print(&output);
	if output.contains("NotFound") {
		context.print("Ariadne parameters not found.");
		Ok(None)
	} else {
		let json: serde_json::Value = serde_json::from_str(&output)?;
		let result: AriadneParametersOutput = serde_json::from_value(json)?;
		let valid_permissioned_candidates: Result<
			Vec<ParsedPermissionedCandidatesKeys>,
			anyhow::Error,
		> = result
			.permissioned_candidates
			.into_iter()
			.filter(|c| c.is_valid)
			.map(TryFrom::<PermissionedCandidateData>::try_from)
			.collect();
		let valid_permissioned_candidates = valid_permissioned_candidates.map_err(|e| {
			anyhow!("Internal error. Could not parse candidate keys from the main chain. {})", e)
		})?;

		Ok(Some(AriadneParameters {
			d_parameter: result.d_parameter,
			permissioned_candidates: SortedPermissionedCandidates::new(
				valid_permissioned_candidates,
			),
		}))
	}
}

fn get_current_mainchain_epoch(
	context: &impl IOContext,
	chain_config: &ChainConfig,
) -> Result<McEpochNumber, anyhow::Error> {
	let mc_epoch_config: sidechain_domain::mainchain_epoch::MainchainEpochConfig =
		From::from(chain_config.cardano.clone());
	mc_epoch_config
		.timestamp_to_mainchain_epoch(context.current_timestamp())
		.map_err(|e| anyhow::anyhow!("{}", e))
}

fn print_on_chain_and_config_permissioned_candidates<C: IOContext>(
	context: &C,
	on_chain_candidates: &SortedPermissionedCandidates,
	config_candidates: &SortedPermissionedCandidates,
) {
	context.print(&format!("Permissioned candidates in the {} file does not match the most recent on-chain initial permissioned candidates.", CHAIN_CONFIG_FILE_PATH));
	context.print("The most recent on-chain initial permissioned candidates are:");
	for candidate in on_chain_candidates.0.iter() {
		context.print(&format!("{}", PermissionedCandidateKeys::from(candidate)));
	}
	context.print("The permissioned candidates in the configuration file are:");
	for candidate in config_candidates.0.iter() {
		context.print(&format!("{}", PermissionedCandidateKeys::from(candidate)));
	}
}

fn set_candidates_on_main_chain<C: IOContext>(
	context: &C,
	candidates: SortedPermissionedCandidates,
	genesis_utxo: UtxoId,
) -> anyhow::Result<Option<ServiceConfig>> {
	let update = context.prompt_yes_no("Do you want to set/update the permissioned candidates on the main chain with values from configuration file?", false);
	if update {
		let ogmios_config = prompt_ogmios_configuration(context)?;
		let payment_signing_key_path =
			CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let pkey =
			cardano_key::get_mc_payment_signing_key_from_file(&payment_signing_key_path, context)?;
		let offchain = context.offchain_impl(&ogmios_config)?;
		let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		tokio_runtime
			.block_on(offchain.upsert_permissioned_candidates(
				genesis_utxo,
				&candidates.to_candidate_data(),
				&pkey,
			))
			.context("Permissioned candidates update failed")?;
		context.print("Permissioned candidates updated. The change will be effective in two main chain epochs.");
		Ok(Some(ogmios_config))
	} else {
		Ok(None)
	}
}

fn set_d_parameter_on_main_chain<C: IOContext>(
	context: &C,
	ogmios_config: Option<ServiceConfig>,
	default_d_parameter: DParameter,
	genesis_utxo: UtxoId,
) -> anyhow::Result<()> {
	let update = context
		.prompt_yes_no("Do you want to set/update the D-parameter on the main chain?", false);
	if update {
		let ogmios_config = match ogmios_config {
			Some(config) => config,
			None => prompt_ogmios_configuration(context)?,
		};
		let p = context.prompt(
			"Enter P, the number of permissioned candidates seats, as a non-negative integer.",
			Some(&default_d_parameter.num_permissioned_candidates.to_string()),
		);
		let num_permissioned_candidates: u16 = p.parse()?;
		let r = context.prompt(
			"Enter R, the number of registered candidates seats, as a non-negative integer.",
			Some(&default_d_parameter.num_registered_candidates.to_string()),
		);
		let num_registered_candidates: u16 = r.parse()?;
		let payment_signing_key_path =
			CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let payment_signing_key =
			cardano_key::get_mc_payment_signing_key_from_file(&payment_signing_key_path, context)?;
		let d_parameter =
			sidechain_domain::DParameter { num_permissioned_candidates, num_registered_candidates };
		let offchain = context.offchain_impl(&ogmios_config)?;
		let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		tokio_runtime.block_on(offchain.upsert_d_param(
			genesis_utxo,
			&d_parameter,
			&payment_signing_key,
		))?;
		context.print(&format!("D-parameter updated to ({}, {}). The change will be effective in two main chain epochs.", p, r));
	}
	Ok(())
}

fn load_chain_config_field<C: IOContext, T: DeserializeOwned>(
	context: &C,
	field: &ConfigFieldDefinition<T>,
) -> Result<T, anyhow::Error> {
	field.load_from_file(context).ok_or_else(|| {
		context.eprint(&format!("The '{}' configuration file is missing or invalid.\nIt should have been created and updated with initial permissioned candidates before running this wizard.", CHAIN_CONFIG_FILE_PATH));
		anyhow!("failed to read '{}'", field.path.join("."))
	})
}
