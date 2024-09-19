use crate::config::config_fields::{CARDANO_PAYMENT_SIGNING_KEY_FILE, POSTGRES_CONNECTION_STRING};
use crate::config::{
	config_fields, get_cardano_network_from_file, ChainConfig, ConfigFieldDefinition,
	SidechainParams, CHAIN_CONFIG_FILE_PATH, PC_CONTRACTS_CLI_PATH,
};
use crate::io::IOContext;
use crate::permissioned_candidates::{ParsedPermissionedCandidatesKeys, PermissionedCandidateKeys};
use crate::pc_contracts_cli_resources::establish_pc_contracts_cli_configuration;
use crate::{smart_contracts, CmdRun};
use anyhow::anyhow;
use epoch_derivation::MainchainEpochDerivation;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sidechain_domain::McEpochNumber;

#[cfg(test)]
mod tests;

#[derive(Debug, clap::Parser)]
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

enum InsertOrUpdate {
	Insert,
	Update,
}

impl InsertOrUpdate {
	fn d_parameter_command(&self) -> &'static str {
		match self {
			InsertOrUpdate::Insert => "insert-d-parameter",
			InsertOrUpdate::Update => "update-d-parameter",
		}
	}

	fn permissioned_candidates_command(&self) -> &'static str {
		"update-permissioned-candidates --remove-all-candidates"
	}
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
}

impl CmdRun for SetupMainChainStateCmd {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		let chain_config = crate::config::load_chain_config(context)?;
		context.print(
			"This wizard will set or update D-Parameter and Permissioned Candidates on the main chain. Setting either of these costs ADA!",
		);
		if !context.file_exists(PC_CONTRACTS_CLI_PATH) {
			return Err(anyhow!(
				"Partner Chains Smart Contracts executable file ({PC_CONTRACTS_CLI_PATH}) is missing",
			));
		}
		let config_initial_authorities =
			initial_permissioned_candidates_from_chain_config(context)?;
		if let Some(ariadne_parameters) = get_ariadne_parameters(context, &chain_config)? {
			if ariadne_parameters.permissioned_candidates == config_initial_authorities {
				context.print(&format!("Permissioned candidates in the {} file match the most recent on-chain initial permissioned candidates.", CHAIN_CONFIG_FILE_PATH));
			} else {
				print_on_chain_and_config_permissioned_candidates(
					context,
					&ariadne_parameters.permissioned_candidates,
					&config_initial_authorities,
				);
				set_candidates_on_main_chain(
					context,
					config_initial_authorities,
					&chain_config.chain_parameters,
					InsertOrUpdate::Update,
				)?;
			}
			context.print(&format!(
				"D-Parameter on the main chain is: (P={}, R={})",
				ariadne_parameters.d_parameter.num_permissioned_candidates,
				ariadne_parameters.d_parameter.num_registered_candidates
			));
			set_d_parameter_on_main_chain(
				context,
				ariadne_parameters.d_parameter,
				&chain_config.chain_parameters,
				InsertOrUpdate::Update,
			)?;
		} else {
			set_candidates_on_main_chain(
				context,
				config_initial_authorities,
				&chain_config.chain_parameters,
				InsertOrUpdate::Insert,
			)?;
			let default_d_parameter =
				DParameter { num_permissioned_candidates: 0, num_registered_candidates: 0 };
			set_d_parameter_on_main_chain(
				context,
				default_d_parameter,
				&chain_config.chain_parameters,
				InsertOrUpdate::Insert,
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
	let executable =
		config_fields::NODE_EXECUTABLE.prompt_with_default_from_file_parse_and_save(context)?;
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
	let mc_epoch_config: epoch_derivation::MainchainEpochConfig =
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
	chain_params: &SidechainParams,
	insert_or_update: InsertOrUpdate,
) -> anyhow::Result<()> {
	let update = context.prompt_yes_no("Do you want to set/update the permissioned candidates on the main chain with values from configuration file?", false);
	if update {
		let pc_contracts_cli_resources = establish_pc_contracts_cli_configuration(context)?;
		let payment_signing_key_path =
			CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let command = insert_or_update.permissioned_candidates_command();
		let candidate_keys = candidates
			.0
			.iter()
			.map(|c| format!("--add-candidate {}", c.to_smart_contracts_args_triple()))
			.collect::<Vec<_>>()
			.join(" ");

		let cardano_network = get_cardano_network_from_file(context)?;

		let output = context.run_command(&format!(
			"{PC_CONTRACTS_CLI_PATH} {} --network {} {} {} {}",
			command,
			cardano_network.to_network_param(),
			candidate_keys,
			smart_contracts::sidechain_params_arguments(chain_params),
			smart_contracts::runtime_config_arguments(
				&pc_contracts_cli_resources,
				&payment_signing_key_path
			)
		))?;
		if output.contains("transactionId") {
			context.print("Permissioned candidates updated. The change will be effective in two main chain epochs.");
			Ok(())
		} else {
			Err(anyhow::anyhow!("Permissioned candidates update failed: {}", output))
		}
	} else {
		Ok(())
	}
}

fn set_d_parameter_on_main_chain<C: IOContext>(
	context: &C,
	default_d_parameter: DParameter,
	chain_params: &SidechainParams,
	insert: InsertOrUpdate,
) -> anyhow::Result<()> {
	let update = context
		.prompt_yes_no("Do you want to set/update the D-parameter on the main chain?", false);
	if update {
		let p = context.prompt(
			"Enter P, the number of permissioned candidates seats, as a non-negative integer.",
			Some(&default_d_parameter.num_permissioned_candidates.to_string()),
		);
		let p: u64 = p.parse()?;
		let r = context.prompt(
			"Enter R, the number of registered candidates seats, as a non-negative integer.",
			Some(&default_d_parameter.num_registered_candidates.to_string()),
		);
		let r: u64 = r.parse()?;
		let pc_contracts_cli_resources = establish_pc_contracts_cli_configuration(context)?;
		let payment_signing_key_path =
			CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let pc_contracts_cli_command = insert.d_parameter_command();
		let cardano_network = get_cardano_network_from_file(context)?;
		let command = format!(
			"{PC_CONTRACTS_CLI_PATH} {pc_contracts_cli_command} --network {} --d-parameter-permissioned-candidates-count {p} --d-parameter-registered-candidates-count {r} {} {}",
			cardano_network.to_network_param(),
			smart_contracts::sidechain_params_arguments(chain_params),
			smart_contracts::runtime_config_arguments(&pc_contracts_cli_resources, &payment_signing_key_path)
		);
		let output = context.run_command(&command)?;
		if output.contains("transactionId") {
			context.print(&format!("D-parameter updated to ({}, {}). The change will be effective in two main chain epochs.", p, r));
			Ok(())
		} else {
			Err(anyhow::anyhow!("Setting D-parameter failed: {}", output))
		}
	} else {
		Ok(())
	}
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
