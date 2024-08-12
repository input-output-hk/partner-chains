pub(crate) mod partner_chain_commands;

use crate::chain_init::{SleeperLive, GENERATED_CHAIN_SPEC_FILE_NAME};
use crate::chain_spec::EnvVarReadError;
use crate::{
	benchmarking::{inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder},
	cli::{Cli, Subcommand},
	service,
};
use crate::{chain_spec, staging, template_chain_spec, testnet};
use epoch_derivation::EpochConfig;
use frame_benchmarking_cli::{BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE};
use futures::{Future, FutureExt};
use sc_cli::{Runner, SubstrateCli};
use sc_service::PartialComponents;
use sidechain_runtime::{Block, EXISTENTIAL_DEPOSIT};
use sp_keyring::Sr25519Keyring;
use sp_session_validator_management_query::commands::*;
use sp_session_validator_management_query::SessionValidatorManagementQuery;

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
			"" => template_chain_spec::chain_spec(),
			path => chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))
				.map_err(|err| EnvVarReadError::ParseError(err.to_string())),
		};

		match maybe_chain_spec {
			Ok(chain_spec) => Ok(Box::new(chain_spec)),
			Err(EnvVarReadError::Missing(err)) => {
				println!("{}", MISSING_ENV_VARIABLES_HELP);
				Err(err)
			},
			Err(EnvVarReadError::ParseError(err)) => Err(err),
		}
	}
}

const MISSING_ENV_VARIABLES_HELP: &str = "Unable to start the node due to missing environment variables. This issue typically occurs when the node executable is launched directly. Instead, please use the `./partner-chains-cli start-node` command, which sets up all the necessary environment variables for you. This command also displays the complete node startup command, including the required environment variables, allowing you to use it directly if needed.";

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
	let cli = Cli::from_args();

	match &cli.subcommand {
		Some(Subcommand::Key(cmd)) => cmd.run(&cli),
		Some(Subcommand::SidechainParams(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial(&config).await?;
				Ok((print_result(sp_sidechain::query::get_sidechain_params(client)), task_manager))
			})
		},
		Some(Subcommand::RegistrationStatus(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, other, .. } =
					service::new_partial(&config).await?;
				let query =
					SessionValidatorManagementQuery::new(client.clone(), other.3.candidate.clone());
				Ok((
					print_result(cli_get_registration_status(
						query,
						cmd.mc_epoch_number,
						cmd.mainchain_pub_key.clone(),
					)),
					task_manager,
				))
			})
		},
		Some(Subcommand::AriadneParameters(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, other, .. } =
					service::new_partial(&config).await?;
				let query =
					SessionValidatorManagementQuery::new(client.clone(), other.3.candidate.clone());
				Ok((
					print_result(cli_get_ariadne_parameters(query, cmd.mc_epoch_number)),
					task_manager,
				))
			})
		},
		Some(Subcommand::RegistrationSignatures(cmd)) => Ok(println!("{}", cmd.execute())),
		Some(Subcommand::BuildSpec(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| async move { cmd.run(config.chain_spec, config.network) })
		},
		Some(Subcommand::CheckBlock(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config).await?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::ExportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial(&config).await?;
				Ok((cmd.run(client, config.database), task_manager))
			})
		},
		Some(Subcommand::ExportState(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, .. } =
					service::new_partial(&config).await?;
				Ok((cmd.run(client, config.chain_spec), task_manager))
			})
		},
		Some(Subcommand::ImportBlocks(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, import_queue, .. } =
					service::new_partial(&config).await?;
				Ok((cmd.run(client, import_queue), task_manager))
			})
		},
		Some(Subcommand::PurgeChain(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| async move { cmd.run(config.database) })
		},
		Some(Subcommand::Revert(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.async_run(|config| async move {
				let PartialComponents { client, task_manager, backend, .. } =
					service::new_partial(&config).await?;
				let aux_revert = Box::new(|client, _, blocks| {
					sc_consensus_grandpa::revert(client, blocks)?;
					Ok(())
				});
				Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
			})
		},
		Some(Subcommand::Benchmark(cmd)) => {
			let runner = cli.create_runner(cmd)?;

			runner.sync_run(|config| async move {
				// This switch needs to be in the client, since the client decides
				// which sub-commands it wants to support.
				match cmd {
					BenchmarkCmd::Pallet(cmd) => {
						if !cfg!(feature = "runtime-benchmarks") {
							return Err(
								"Runtime benchmarking wasn't enabled when building the node. \
							You can enable it with `--features runtime-benchmarks`."
									.into(),
							);
						}

						cmd.run_with_spec::<sp_runtime::traits::HashingFor<Block>, ()>(Some(
							config.chain_spec,
						))
					},
					BenchmarkCmd::Block(cmd) => {
						let PartialComponents { client, .. } =
							service::new_partial(&config).await?;
						cmd.run(client)
					},
					#[cfg(not(feature = "runtime-benchmarks"))]
					BenchmarkCmd::Storage(_) => Err(
						"Storage benchmarking can be enabled with `--features runtime-benchmarks`."
							.into(),
					),
					#[cfg(feature = "runtime-benchmarks")]
					BenchmarkCmd::Storage(cmd) => {
						let PartialComponents { client, backend, .. } =
							service::new_partial(&config).await?;
						let db = backend.expose_db();
						let storage = backend.expose_storage();

						cmd.run(config, client, db, storage)
					},
					BenchmarkCmd::Overhead(cmd) => {
						let PartialComponents { client, .. } =
							service::new_partial(&config).await?;
						let ext_builder = RemarkBuilder::new(client.clone());

						cmd.run(
							config,
							client,
							inherent_benchmark_data()?,
							Vec::new(),
							&ext_builder,
						)
					},
					BenchmarkCmd::Extrinsic(cmd) => {
						let PartialComponents { client, .. } =
							service::new_partial(&config).await?;
						// Register the *Remark* and *TKA* builders.
						let ext_factory = ExtrinsicFactory(vec![
							Box::new(RemarkBuilder::new(client.clone())),
							Box::new(TransferKeepAliveBuilder::new(
								client.clone(),
								Sr25519Keyring::Alice.to_account_id(),
								EXISTENTIAL_DEPOSIT,
							)),
						]);

						cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
					},
					BenchmarkCmd::Machine(cmd) => {
						cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone())
					},
				}
			})
		},
		Some(Subcommand::ChainInfo(cmd)) => {
			let runner = cli.create_runner(cmd)?;
			runner.sync_run(|config| async move { cmd.run::<Block>(&config) })
		},
		None => {
			let use_chain_init = read_chain_init_flag();

			let mut runner = cli.create_runner(&cli.run)?;

			if use_chain_init {
				let chain_spec = match crate::chain_init::read_spec(GENERATED_CHAIN_SPEC_FILE_NAME)
				{
					Ok(spec) => {
						// Excessive logging to avoid confusion
						log::info!("💡{} has been found in the working directory, and it will be used as the chain specification for the node. This is because the USE_CHAIN_INIT flag is currently set to true. If you wish to use a custom chain specification, please disable the USE_CHAIN_INIT flag.", GENERATED_CHAIN_SPEC_FILE_NAME);
						spec
					},
					Err(_) => {
						log::info!("🏗️ Starting chain initialization procedure...");
						initialize_chain_spec(&runner)?
					},
				};
				// Update the runner config with the chain spec and run the node
				runner.config_mut().chain_spec = chain_spec;
				log::info!("Starting the node with the generated chain spec");
			}

			runner.run_node_until_exit(|config| async move {
				service::new_full::<sc_network::NetworkWorker<_, _>>(config)
					.await
					.map_err(sc_cli::Error::Service)
			})
		},
	}
}

fn read_chain_init_flag() -> bool {
	std::env::var("USE_CHAIN_INIT")
		.ok()
		.and_then(|val| val.parse::<bool>().ok())
		.unwrap_or(false)
}

fn initialize_chain_spec(runner: &Runner<Cli>) -> Result<Box<dyn sc_service::ChainSpec>, String> {
	let chain_init_client_future = service::chain_init_client(&runner.config).map(|res| res.ok());
	let (client, data_sources) = runner
		.interruptible_block_on(chain_init_client_future)
		.map_err(|err| format!("Failed to create client: {:?}", err))?;

	let epoch_config =
		EpochConfig::read().map_err(|err| format!("Failed to read epoch config: {:?}", err))?;

	let generated_chain_spec_future =
		crate::chain_init::run(client, &data_sources, &epoch_config, runner.config(), SleeperLive)
			.map(|res| res.ok());

	runner
		.interruptible_block_on(generated_chain_spec_future)
		.map(|generated_chain_spec| {
			log::info!("✅ Chain initialization success, the initial committee has been selected");
			log::info!("💾 Saving the generated chain spec to {}", GENERATED_CHAIN_SPEC_FILE_NAME);
			let _ = crate::chain_init::save_spec(
				generated_chain_spec.cloned_box(),
				GENERATED_CHAIN_SPEC_FILE_NAME,
			)
			.map_err(|err| {
				log::error!("Failed to save the generated chain spec: {:?}", err);
			});

			generated_chain_spec
		})
		.map_err(|_| "Initialization process terminated by user".into())
}

async fn print_result<FIn>(command_future: FIn) -> Result<(), sc_cli::Error>
where
	FIn: Future<Output = Result<String, String>>,
{
	let result = command_future.await?;
	println!("{}", result);
	Ok(())
}
