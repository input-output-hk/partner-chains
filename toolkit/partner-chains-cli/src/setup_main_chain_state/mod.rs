use crate::config::config_fields::CARDANO_PAYMENT_SIGNING_KEY_FILE;
use crate::config::{CHAIN_CONFIG_FILE_PATH, ChainConfig, ConfigFieldDefinition, config_fields};
use crate::io::IOContext;
use crate::ogmios::config::prompt_ogmios_configuration;
use crate::permissioned_candidates::{ParsedPermissionedCandidatesKeys, PermissionedCandidateKeys};
use crate::{CmdRun, cardano_key};
use anyhow::Context;
use anyhow::anyhow;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::d_param::{GetDParam, UpsertDParam};
use partner_chains_cardano_offchain::multisig::{
	MultiSigSmartContractResult, MultiSigTransactionData,
};
use partner_chains_cardano_offchain::permissioned_candidates::{
	GetPermissionedCandidates, UpsertPermissionedCandidates,
};
use serde::de::DeserializeOwned;
use sidechain_domain::{DParameter, PermissionedCandidateData, UtxoId};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, clap::Parser)]
pub struct SetupMainChainStateCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
}

impl TryFrom<PermissionedCandidateData> for ParsedPermissionedCandidatesKeys {
	type Error = anyhow::Error;

	fn try_from(value: PermissionedCandidateData) -> Result<Self, Self::Error> {
		let keys = PermissionedCandidateKeys {
			sidechain_pub_key: hex::encode(value.sidechain_public_key.0),
			aura_pub_key: hex::encode(value.aura_public_key.0),
			grandpa_pub_key: hex::encode(value.grandpa_public_key.0),
		};
		TryFrom::try_from(&keys)
	}
}

#[derive(Debug, PartialEq)]
struct SortedPermissionedCandidates(Vec<PermissionedCandidateData>);

impl SortedPermissionedCandidates {
	pub fn new(mut keys: Vec<PermissionedCandidateData>) -> Self {
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
		let config_initial_authorities =
			initial_permissioned_candidates_from_chain_config(context)?;
		context.print("Will read the current D-Parameter and Permissioned Candidates from the main chain using Ogmios client.");
		let ogmios_config = prompt_ogmios_configuration(context)?;
		let offchain = context.offchain_impl(&ogmios_config)?;

		match get_permissioned_candidates::<C>(&offchain, &chain_config)? {
			Some(candidates) if candidates == config_initial_authorities => {
				context.print(&format!("Permissioned candidates in the {} file match the most recent on-chain initial permissioned candidates.", CHAIN_CONFIG_FILE_PATH));
			},
			candidates => {
				print_on_chain_and_config_permissioned_candidates(
					context,
					candidates,
					&config_initial_authorities,
				);
				set_candidates_on_main_chain(
					self.common_arguments.retries(),
					context,
					&offchain,
					config_initial_authorities,
					chain_config.chain_parameters.genesis_utxo,
				)?;
			},
		};
		let d_parameter = get_d_parameter::<C>(&offchain, &chain_config)?;
		print_on_chain_d_parameter(context, &d_parameter);
		set_d_parameter_on_main_chain(
			self.common_arguments.retries(),
			context,
			&offchain,
			d_parameter.unwrap_or(DParameter {
				num_permissioned_candidates: 0,
				num_registered_candidates: 0,
			}),
			chain_config.chain_parameters.genesis_utxo,
		)?;
		context.print("Done. Please remember that any changes to the Cardano state can be observed immediately, but from the Partner Chain point of view they will be effective in two main chain epochs.");
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
	// Use ParsedPermissionedCandidatesKeys to validate them
	let candidates = candidates
		.iter()
		.map(ParsedPermissionedCandidatesKeys::try_from)
		.collect::<Result<Vec<_>, _>>()?;
	let candidates = candidates.iter().map(PermissionedCandidateData::from).collect();
	Ok(SortedPermissionedCandidates::new(candidates))
}

fn get_permissioned_candidates<C: IOContext>(
	offchain: &C::Offchain,
	chain_config: &ChainConfig,
) -> anyhow::Result<Option<SortedPermissionedCandidates>> {
	let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
	let candidates_opt = tokio_runtime
		.block_on(offchain.get_permissioned_candidates(chain_config.chain_parameters.genesis_utxo))
		.context("Failed to read Permissioned Candidates from Ogmios")?;
	Ok(candidates_opt.map(|candidates| SortedPermissionedCandidates::new(candidates)))
}

fn get_d_parameter<C: IOContext>(
	offchain: &C::Offchain,
	chain_config: &ChainConfig,
) -> anyhow::Result<Option<DParameter>> {
	let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
	let d_param_opt = tokio_runtime
		.block_on(offchain.get_d_param(chain_config.chain_parameters.genesis_utxo))
		.context("Failed to get D-parameter from Ogmios")?;
	Ok(d_param_opt)
}

fn print_on_chain_and_config_permissioned_candidates<C: IOContext>(
	context: &C,
	on_chain_candidates: Option<SortedPermissionedCandidates>,
	config_candidates: &SortedPermissionedCandidates,
) {
	match on_chain_candidates {
		Some(candidates) => {
			context.print(&format!("Permissioned candidates in the {} file does not match the most recent on-chain initial permissioned candidates.", CHAIN_CONFIG_FILE_PATH));
			context.print("The most recent on-chain initial permissioned candidates are:");
			for candidate in candidates.0.iter() {
				context.print(&format!("{}", PermissionedCandidateKeys::from(candidate)));
			}
			context.print("The permissioned candidates in the configuration file are:");
			for candidate in config_candidates.0.iter() {
				context.print(&format!("{}", PermissionedCandidateKeys::from(candidate)));
			}
		},
		None => context.print("List of permissioned candidates is not set on Cardano yet."),
	}
}

fn print_on_chain_d_parameter<C: IOContext>(
	context: &C,
	on_chain_d_parameter: &Option<DParameter>,
) {
	if let Some(d_parameter) = on_chain_d_parameter {
		context.print(&format!(
			"D-Parameter on the main chain is: (P={}, R={})",
			d_parameter.num_permissioned_candidates, d_parameter.num_registered_candidates
		))
	}
}

fn set_candidates_on_main_chain<C: IOContext>(
	await_tx: FixedDelayRetries,
	context: &C,
	offchain: &C::Offchain,
	candidates: SortedPermissionedCandidates,
	genesis_utxo: UtxoId,
) -> anyhow::Result<()> {
	let update = context.prompt_yes_no("Do you want to set/update the permissioned candidates on the main chain with values from configuration file?", false);
	if update {
		let payment_signing_key_path =
			CARDANO_PAYMENT_SIGNING_KEY_FILE.prompt_with_default_from_file_and_save(context);
		let pkey =
			cardano_key::get_mc_payment_signing_key_from_file(&payment_signing_key_path, context)?;
		let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		let result = tokio_runtime
			.block_on(offchain.upsert_permissioned_candidates(
				await_tx,
				genesis_utxo,
				&candidates.0,
				&pkey,
			))
			.context("Permissioned candidates update failed")?;
		match result {
			None => context.print(
				"Permissioned candidates on the Cardano are already equal to value from the config file.",
			),
			Some(MultiSigSmartContractResult::TransactionSubmitted(_)) => context.print(
				"Permissioned candidates updated. The change will be effective in two main chain epochs.",
			),
			Some(MultiSigSmartContractResult::TransactionToSign(tx_data)) => {
				print_tx_to_sign_and_instruction(
					context,
					"update permissioned candidates",
					&tx_data,
				)?
			},
		}
	}
	Ok(())
}

fn set_d_parameter_on_main_chain<C: IOContext>(
	await_tx: FixedDelayRetries,
	context: &C,
	offchain: &C::Offchain,
	default_d_parameter: DParameter,
	genesis_utxo: UtxoId,
) -> anyhow::Result<()> {
	let update = context
		.prompt_yes_no("Do you want to set/update the D-parameter on the main chain?", false);
	if update {
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
		let tokio_runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		let result = tokio_runtime.block_on(offchain.upsert_d_param(
			await_tx,
			genesis_utxo,
			&d_parameter,
			&payment_signing_key,
		))?;
		match result {
			None => context.print(&format!("D-parameter is set to ({}, {}) already.", p, r)),
			Some(MultiSigSmartContractResult::TransactionSubmitted(_)) => context.print(&format!(
				"D-parameter updated to ({}, {}). The change will be effective in two main chain epochs.",
				p, r
			)),
			Some(MultiSigSmartContractResult::TransactionToSign(tx_data)) => {
				print_tx_to_sign_and_instruction(context, "update D-parameter", &tx_data)?
			},
		}
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

fn print_tx_to_sign_and_instruction<C: IOContext>(
	context: &C,
	tx_name: &str,
	tx: &MultiSigTransactionData,
) -> anyhow::Result<()> {
	let json = serde_json::to_string_pretty(&tx)?;
	context.print(&format!(
		"The Partner chain is governed by MultiSig. Sign and submit the {tx_name} transaction:"
	));
	context.print(&json);
	context.print("Please find the instructions at: https://github.com/input-output-hk/partner-chains/blob/master/docs/user-guides/governance/governance.md#multi-signature-governance");
	Ok(())
}
