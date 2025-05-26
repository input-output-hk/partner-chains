//! # Partner Chains Node Commands
//!
//! This crate provides a comprehensive set of command-line interface (CLI) commands for
//! interacting with Partner Chains nodes. It serves as the primary interface for node
//! operators, validators, and developers to manage and interact with Partner Chain infrastructure.
//!
//! ## Overview
//!
//! Partner Chains are application-specific blockchains that leverage Cardano's security and
//! decentralization while providing enhanced functionality and performance for specific use cases.
//! This crate provides the essential CLI tooling to:
//!
//! - Query sidechain parameters and status
//! - Manage validator registration and committee membership
//! - Handle cryptographic signatures for various operations
//! - Interact with Partner Chain smart contracts on Cardano
//! - Access setup wizards for chain configuration
//!
//! ## Architecture
//!
//! The crate is built around the [`PartnerChainsSubcommand`] enum, which defines all available
//! commands. Each command is implemented as a separate struct with its own configuration and
//! execution logic. The main entry point is the [`run`] function, which dispatches commands
//! to their appropriate handlers.
//!
//! ## Usage
//!
//! ```rust,no_run
//! use partner_chains_node_commands::{PartnerChainsSubcommand, run};
//!
//! // In your main CLI handler
//! match cli.subcommand {
//!     Some(subcommand) => run(&cli, get_deps, subcommand),
//!     None => // handle default case
//! }
//! ```

use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionDataSource;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi;
use clap::Parser;
use cli_commands::address_association_signatures::AddressAssociationSignaturesCmd;
use cli_commands::block_producer_metadata_signatures::BlockProducerMetadataSignatureCmd;
use cli_commands::registration_signatures::RegistrationSignaturesCmd;
use frame_support::sp_runtime::traits::NumberFor;
use parity_scale_codec::{Decode, Encode};
use partner_chains_cli::io::DefaultCmdRunContext;
pub use partner_chains_cli::{
	PartnerChainRuntime, PartnerChainRuntimeBindings, RuntimeTypeWrapper,
};
use partner_chains_smart_contracts_commands::SmartContractsCmd;
use sc_cli::{CliConfiguration, SharedParams, SubstrateCli};
use sc_service::TaskManager;
use sidechain_domain::{McEpochNumber, ScEpochNumber, StakePoolPublicKey};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::DeserializeOwned;
use sp_runtime::Serialize;
use sp_runtime::traits::Block as BlockT;
use sp_session_validator_management::CommitteeMember as CommitteeMemberT;
use sp_session_validator_management::SessionValidatorManagementApi;
use sp_session_validator_management_query::SessionValidatorManagementQuery;
use sp_session_validator_management_query::commands::*;
#[allow(deprecated)]
use sp_sidechain::{GetGenesisUtxo, GetSidechainStatus};
use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;

/// Command for retrieving Ariadne protocol parameters effective at a specific mainchain epoch.
///
/// Ariadne parameters control various aspects of the Partner Chain consensus mechanism.
/// These parameters become effective two epochs after the block containing their change
/// is included in the mainchain.
///
/// # Example
///
/// ```bash
/// partner-chains-node ariadne-parameters --mc-epoch-number 586
/// ```
#[derive(Debug, Clone, Parser)]
pub struct AriadneParametersCmd {
	/// The mainchain epoch number for which to retrieve Ariadne parameters
	#[arg(long)]
	pub mc_epoch_number: McEpochNumber,
	/// Shared CLI parameters for configuration
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl CliConfiguration for AriadneParametersCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}

/// Command for retrieving fundamental sidechain parameters.
///
/// This command returns essential sidechain configuration including genesis UTXO
/// and other foundational parameters required for sidechain operation.
///
/// # Example
///
/// ```bash
/// partner-chains-node sidechain-params
/// ```
#[derive(Debug, Clone, Parser)]
pub struct SidechainParamsCmd {
	/// Shared CLI parameters for configuration
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl CliConfiguration for SidechainParamsCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}

/// Command for checking validator registration status on the Partner Chain.
///
/// This command verifies whether a validator's registration has been successfully
/// processed and is recognized by the Partner Chain. It's essential for validator
/// operators to confirm their registration before attempting to participate in
/// committee selection.
///
/// # Important Notes
///
/// - If registration was included in Cardano block in epoch N, it should be visible
///   when querying epoch N+1 or later
/// - If registration doesn't appear after a few minutes, debugging may be required
/// - The `mainchain-pub-key` alias is provided for backward compatibility
///
/// # Example
///
/// ```bash
/// partner-chains-node registration-status \
///   --stake-pool-pub-key 0x702b81ab2e86cf73a87062af1eb0da666d451976d9d91c63a119ed94e6a33dc0 \
///   --mc-epoch-number 586
/// ```
#[derive(Debug, Clone, Parser)]
pub struct RegistrationStatusCmd {
	/// The stake pool public key (mainchain public key) to check registration for
	#[arg(long, alias = "mainchain-pub-key")]
	pub stake_pool_pub_key: StakePoolPublicKey,
	/// The mainchain epoch number to check registration status at
	#[arg(long)]
	pub mc_epoch_number: McEpochNumber,
	/// Shared CLI parameters for configuration
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl CliConfiguration for RegistrationStatusCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}

/// Enumeration of all available Partner Chains node commands.
///
/// This enum defines the complete set of commands available for Partner Chain operations.
/// Each variant corresponds to a specific command with its own parameters and functionality.
/// The enum is generic over runtime bindings and address types to support different
/// Partner Chain configurations.
///
/// # Type Parameters
///
/// - `RuntimeBindings`: Partner Chain runtime bindings implementing required traits
/// - `PartnerchainAddress`: Address format specific to the Partner Chain implementation
///
/// # Commands
///
/// - [`SidechainParams`]: Query fundamental sidechain parameters
/// - [`RegistrationStatus`]: Check validator registration status
/// - [`AriadneParameters`]: Retrieve Ariadne protocol parameters
/// - [`RegistrationSignatures`]: Generate registration signatures
/// - [`SignAddressAssociation`]: Sign address associations
/// - [`SignBlockProducerMetadata`]: Sign block producer metadata
/// - [`SmartContracts`]: Interact with smart contracts on Cardano
/// - [`Wizards`]: Access interactive setup wizards
#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum PartnerChainsSubcommand<
	RuntimeBindings: PartnerChainRuntime + PartnerChainRuntimeBindings,
	PartnerchainAddress: Clone + Sync + Send + FromStr + 'static,
> {
	/// Returns sidechain parameters including genesis UTXO and configuration
	SidechainParams(SidechainParamsCmd),

	/// Returns registration status for a given mainchain public key and epoch number.
	/// If registration has been included in Cardano block in epoch N, then it should be returned by this command if epoch greater than N+1 is provided.
	/// If this command won't show your registration after a few minutes after it has been included in a cardano block, you can start debugging for unsuccessful registration.
	#[clap(
		after_help = "Example: partner-chains-node -- registration-status --stake-pool-pub-key 0x702b81ab2e86cf73a87062af1eb0da666d451976d9d91c63a119ed94e6a33dc0 --mc-epoch-number 586"
	)]
	RegistrationStatus(RegistrationStatusCmd),

	/// Returns ariadne parameters effective at given mainchain epoch number.
	/// Parameters are effective two epochs after the block their change is included in.
	AriadneParameters(AriadneParametersCmd),

	/// Generates registration signatures for partner chains committee candidates
	RegistrationSignatures(RegistrationSignaturesCmd),

	/// Signs address association between different chain contexts
	SignAddressAssociation(AddressAssociationSignaturesCmd<PartnerchainAddress>),

	/// Signs block producer metadata for submitting to the runtime
	SignBlockProducerMetadata(BlockProducerMetadataSignatureCmd),

	/// Commands for interacting with Partner Chain smart contracts on Cardano
	#[command(subcommand)]
	SmartContracts(SmartContractsCmd),

	/// Partner Chains text "wizards" for setting up chain configuration
	#[command(subcommand)]
	Wizards(partner_chains_cli::Command<RuntimeBindings>),
}

/// Main command execution function for Partner Chains node commands.
///
/// This function serves as the primary entry point for executing Partner Chain CLI commands.
/// It takes a CLI interface, dependency provider, and a specific command, then dispatches
/// the command to its appropriate handler.
///
/// # Type Parameters
///
/// - `Cli`: The CLI interface implementing [`SubstrateCli`]
/// - `Block`: The blockchain block type implementing [`BlockT`]
/// - `CommitteeMember`: Committee member type for validator management
/// - `Client`: Blockchain client providing runtime API access
/// - `BlockProducerMetadata`: Metadata type for block producer operations
/// - `RuntimeBindings`: Partner Chain runtime bindings
/// - `PartnerchainAddress`: Address format for the specific Partner Chain
///
/// # Arguments
///
/// - `cli`: Reference to the CLI interface
/// - `get_deps`: Closure that provides necessary dependencies (client, task manager, data source)
/// - `cmd`: The specific Partner Chain command to execute
///
/// # Returns
///
/// Returns `sc_cli::Result<()>` indicating success or failure of command execution.
///
/// # Example
///
/// ```rust,no_run
/// use partner_chains_node_commands::{PartnerChainsSubcommand, run};
/// 
/// let result = run(&cli, get_dependencies, command);
/// match result {
///     Ok(()) => println!("Command executed successfully"),
///     Err(e) => eprintln!("Command failed: {}", e),
/// }
/// ```
#[allow(deprecated)]
pub fn run<
	Cli,
	Block,
	CommitteeMember,
	Client,
	BlockProducerMetadata,
	RuntimeBindings: PartnerChainRuntime + PartnerChainRuntimeBindings,
	PartnerchainAddress,
>(
	cli: &Cli,
	get_deps: impl FnOnce(
		sc_service::Configuration,
	) -> Result<
		(Arc<Client>, TaskManager, Arc<dyn AuthoritySelectionDataSource + Send + Sync>),
		sc_service::error::Error,
	>,
	cmd: PartnerChainsSubcommand<RuntimeBindings, PartnerchainAddress>,
) -> sc_cli::Result<()>
where
	Cli: SubstrateCli,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	Client::Api: GetGenesisUtxo<Block>
		+ GetSidechainStatus<Block>
		+ SessionValidatorManagementApi<
			Block,
			CommitteeMember,
			AuthoritySelectionInputs,
			ScEpochNumber,
		> + CandidateValidationApi<Block>,
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
	CommitteeMember: CommitteeMemberT + Encode + Decode + Send + Sync + 'static,
	CommitteeMember::AuthorityId: Decode + Encode + AsRef<[u8]> + Send + Sync + 'static,
	CommitteeMember::AuthorityKeys: Decode + Encode,
	BlockProducerMetadata: DeserializeOwned + Encode + Send + Sync,
	PartnerchainAddress: Serialize + Clone + Sync + Send + FromStr + Encode + 'static,
{
	match cmd {
		PartnerChainsSubcommand::SidechainParams(cmd) => {
			let runner = cli.create_runner(&cmd)?;
			runner.async_run(|config| {
				let (client, task_manager, _) = get_deps(config)?;
				Ok((print_result(cli_commands::get_genesis_utxo::execute(client)), task_manager))
			})
		},
		PartnerChainsSubcommand::RegistrationStatus(cmd) => {
			let runner = cli.create_runner(&cmd)?;
			runner.async_run(move |config| {
				let (client, task_manager, ds) = get_deps(config)?;
				let query = SessionValidatorManagementQuery::new(client.clone(), ds.clone());
				Ok((
					print_result(cli_get_registration_status(
						query,
						cmd.mc_epoch_number,
						cmd.stake_pool_pub_key.clone(),
					)),
					task_manager,
				))
			})
		},
		PartnerChainsSubcommand::AriadneParameters(cmd) => {
			let runner = cli.create_runner(&cmd)?;
			runner.async_run(move |config| {
				let (client, task_manager, ds) = get_deps(config)?;
				let query = SessionValidatorManagementQuery::new(client.clone(), ds.clone());
				Ok((
					print_result(cli_get_ariadne_parameters(query, cmd.mc_epoch_number)),
					task_manager,
				))
			})
		},
		PartnerChainsSubcommand::RegistrationSignatures(cmd) => Ok(println!("{}", cmd.execute())),
		PartnerChainsSubcommand::SignAddressAssociation(cmd) => {
			cmd.execute().map_err(|e| sc_service::Error::Application(e.into()))?;
			Ok(())
		},
		PartnerChainsSubcommand::SignBlockProducerMetadata(cmd) => {
			cmd.execute::<BlockProducerMetadata>()
				.map_err(|e| sc_service::Error::Application(e.into()))?;
			Ok(())
		},
		PartnerChainsSubcommand::SmartContracts(cmd) => {
			setup_log4rs()?;
			Ok(cmd.execute_blocking()?)
		},
		PartnerChainsSubcommand::Wizards(cmd) => {
			setup_log4rs()?;
			Ok(cmd
				.run(&DefaultCmdRunContext)
				.map_err(|e| sc_cli::Error::Application(e.into()))?)
		},
	}
}

/// Configures logging for Partner Chains commands with specialized routing.
///
/// This function sets up a sophisticated logging configuration that:
/// - Routes general logs to stderr to keep stdout clean for JSON outputs
/// - Creates a dedicated log file for Ogmios client interactions
/// - Configures appropriate log levels for different components
///
/// This separation is crucial for smart contract commands that output JSON
/// to stdout, ensuring that log messages don't interfere with the structured output.
///
/// # Log Configuration
///
/// - **stderr**: General application logs at INFO level
/// - **ogmios_client.log**: Ogmios client interactions at DEBUG level
///
/// # Errors
///
/// Returns an error if the logging configuration cannot be initialized.
///
/// # Example
///
/// ```rust,no_run
/// setup_log4rs().expect("Failed to setup logging");
/// ```
fn setup_log4rs() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let stderr = log4rs::append::console::ConsoleAppender::builder()
		.target(log4rs::append::console::Target::Stderr)
		.build();
	let ogmios_log = log4rs::append::file::FileAppender::builder().build("ogmios_client.log")?;

	let log_config = log4rs::config::Config::builder()
		.appender(log4rs::config::Appender::builder().build("stderr", Box::new(stderr)))
		.appender(log4rs::config::Appender::builder().build("ogmios-log", Box::new(ogmios_log)))
		.logger(
			log4rs::config::Logger::builder()
				.appender("ogmios-log")
				.additive(false)
				.build("ogmios_client::jsonrpsee", log::LevelFilter::Debug),
		)
		.build(log4rs::config::Root::builder().appender("stderr").build(log::LevelFilter::Info))?;

	log4rs::init_config(log_config)?;

	Ok(())
}

/// Utility function for printing command execution results.
///
/// This function awaits the result of an asynchronous command future and prints
/// the result to stdout. It handles both success and error cases gracefully,
/// ensuring that error messages are displayed to the user.
///
/// # Arguments
///
/// - `command_future`: A future that resolves to either a success string or error string
///
/// # Returns
///
/// Always returns `Ok(())` since errors are printed rather than propagated.
///
/// # Example
///
/// ```rust,no_run
/// use partner_chains_node_commands::print_result;
///
/// async fn example_command() -> Result<String, String> {
///     Ok("Command completed successfully".to_string())
/// }
///
/// let result = print_result(example_command()).await;
/// assert!(result.is_ok());
/// ```
async fn print_result<FIn>(command_future: FIn) -> Result<(), sc_cli::Error>
where
	FIn: Future<Output = Result<String, String>>,
{
	let result = command_future.await.unwrap_or_else(|e| e);
	println!("{}", result);
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	async fn some_err() -> Result<String, String> {
		Err("some err".to_string())
	}

	#[tokio::test]
	async fn print_async_doesnt_fail_if_result_is_error() {
		let result = print_result(some_err()).await;
		assert!(result.is_ok());
	}
}
