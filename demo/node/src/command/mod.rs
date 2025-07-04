use crate::cli::WizardBindings;
use crate::{chain_spec, staging, template_chain_spec, testnet};
use crate::{
	cli::{Cli, Subcommand},
	service,
};
use partner_chains_demo_runtime::{Block, BlockProducerMetadataType};
use sc_cli::SubstrateCli;
use sc_service::PartialComponents;
use sp_runtime::AccountId32;

impl SubstrateCli for Cli {
	fn impl_name() -> String {
		"Partner Chains Node".into()
	}

	fn impl_version() -> String {
		env!("SUBSTRATE_CLI_IMPL_VERSION").into()
	}

	fn description() -> String {
		env!("CARGO_PKG_DESCRIPTION").into()
	}

	fn author() -> String {
		env!("CARGO_PKG_AUTHORS").into()
	}

	fn support_url() -> String {
		"support.anonymous.an".into()
	}

	fn copyright_start_year() -> i32 {
		2017
	}

	fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
		let maybe_chain_spec = match id {
			"dev" => testnet::development_config(),
			"staging" => staging::staging_config(),
			"local" => testnet::local_testnet_config(),
			"template" => template_chain_spec::chain_spec(),
			"" => {
				return Err("Please provide --chain dev|staging|local|template|<path> parameter"
					.to_string());
			},
			path => match chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path)) {
				Ok(parsed) => Ok(parsed),
				Err(err) => return Err(format!("Parsing chain spec file failed: {}", err)),
			},
		};

		match maybe_chain_spec {
			Ok(chain_spec) => Ok(Box::new(chain_spec)),
			Err(err) => {
				println!("{}", INVALID_ENV_VARIABLES_HELP);
				Err(format!("Reading configuration from environment failed: {}", err))
			},
		}
	}
}

const INVALID_ENV_VARIABLES_HELP: &str = "Unable to start the node due to missing or malformed environment variables. This issue typically occurs when the node executable is launched directly. Instead, please use the `./partner-chains-cli start-node` command, which sets up all the necessary environment variables for you. This command also displays the complete node startup command, including the required environment variables, allowing you to use it directly if needed.";

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::PartnerChains(cmd)) => {
			let make_dependencies = |config| {
				let components = service::new_partial(&config)?;
				Ok((
					components.client,
					components.task_manager,
					components.other.3.authority_selection,
				))
			};
			partner_chains_node_commands::run::<
				_,
				_,
				_,
				_,
				BlockProducerMetadataType,
				WizardBindings,
				AccountId32,
			>(&cli, make_dependencies, cmd.clone())
		},
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial(&config)?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, .. } = service::new_partial(&config)?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config)?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run(config.database))
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| {
				let PartialComponents { client, task_manager, backend, .. } =
					service::new_partial(&config)?;
				let aux_revert = Box::new(|client, _, blocks| {
					sc_consensus_grandpa::revert(client, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| cmd.run::<Block>(&config))
		},
		None => {
			let runner = cli.create_runner(&cli.run)?;
			runner.run_node_until_exit(|config| async move {
				service::new_full(config).await.map_err(sc_cli::Error::Service)
			})
		},
	}
}
