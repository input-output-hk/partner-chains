use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionDataSource;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi;
use clap::Parser;
use cli_commands::address_association_signatures::AddressAssociationSignaturesCmd;
use cli_commands::registration_signatures::RegistrationSignaturesCmd;
use frame_support::sp_runtime::traits::NumberFor;
use parity_scale_codec::{Decode, Encode};
use partner_chains_cli::io::DefaultCmdRunContext;
use partner_chains_smart_contracts_commands::SmartContractsCmd;
use sc_cli::{CliConfiguration, SharedParams, SubstrateCli};
use sc_service::TaskManager;
use sidechain_domain::{McEpochNumber, ScEpochNumber, StakePoolPublicKey};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::AccountId32;
use sp_session_validator_management::SessionValidatorManagementApi;
use sp_session_validator_management_query::commands::*;
use sp_session_validator_management_query::SessionValidatorManagementQuery;
use sp_sidechain::GetGenesisUtxo;
use sp_sidechain::GetSidechainStatus;
use std::future::Future;
use std::sync::Arc;

#[derive(Debug, Clone, Parser)]
pub struct AriadneParametersCmd {
	#[arg(long)]
	pub mc_epoch_number: McEpochNumber,
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl CliConfiguration for AriadneParametersCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}

#[derive(Debug, Clone, Parser)]
pub struct SidechainParamsCmd {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl CliConfiguration for SidechainParamsCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}

#[derive(Debug, Clone, Parser)]
pub struct RegistrationStatusCmd {
	#[arg(long)]
	#[arg(long, alias = "mainchain-pub-key")]
	pub stake_pool_pub_key: StakePoolPublicKey,
	#[arg(long)]
	pub mc_epoch_number: McEpochNumber,
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub shared_params: SharedParams,
}

impl CliConfiguration for RegistrationStatusCmd {
	fn shared_params(&self) -> &SharedParams {
		&self.shared_params
	}
}

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum PartnerChainsSubcommand {
	/// Returns sidechain parameters
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

	/// Signs address association
	SignAddressAssociation(AddressAssociationSignaturesCmd<AccountId32>),

	/// Commands for interacting with Partner Chain smart contracts on Cardano
	#[command(subcommand)]
	SmartContracts(SmartContractsCmd),

	/// Partner Chains text "wizards" for setting up chain
	#[command(subcommand)]
	Wizards(partner_chains_cli::Command),
}

pub fn run<Cli, Block, CrossChainPublic, SessionKeys, Client>(
	cli: &Cli,
	get_deps: impl FnOnce(
		sc_service::Configuration,
	) -> Result<
		(Arc<Client>, TaskManager, Arc<dyn AuthoritySelectionDataSource + Send + Sync>),
		sc_service::error::Error,
	>,
	cmd: PartnerChainsSubcommand,
) -> sc_cli::Result<()>
where
	Cli: SubstrateCli,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + 'static,
	Client::Api: GetGenesisUtxo<Block>
		+ GetSidechainStatus<Block>
		+ SessionValidatorManagementApi<
			Block,
			SessionKeys,
			CrossChainPublic,
			AuthoritySelectionInputs,
			ScEpochNumber,
		> + CandidateValidationApi<Block>,
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
	SessionKeys: Decode + Send + Sync + 'static,
	CrossChainPublic: Decode + Encode + AsRef<[u8]> + Send + Sync + 'static,
{
	match cmd {
		PartnerChainsSubcommand::SidechainParams(cmd) => {
			let runner = cli.create_runner(&cmd)?;
			runner.async_run(|config| {
				let (client, task_manager, _) = get_deps(config)?;
				Ok((print_result(sp_sidechain::query::get_genesis_utxo(client)), task_manager))
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
		PartnerChainsSubcommand::SmartContracts(cmd) => {
			crate::setup_log4rs()?;
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

/// This sets logging in a very opinionated way.
/// Because Rust env_logger clashes with log4rs, this is exposed to be invoked by users of smart-contracts commands.
pub fn setup_log4rs() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	let stdout = log4rs::append::console::ConsoleAppender::builder().build();
	let ogmios_log = log4rs::append::file::FileAppender::builder().build("ogmios_client.log")?;

	let log_config = log4rs::config::Config::builder()
		.appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
		.appender(log4rs::config::Appender::builder().build("ogmios-log", Box::new(ogmios_log)))
		.logger(
			log4rs::config::Logger::builder()
				.appender("ogmios-log")
				.additive(false)
				.build("ogmios_client::jsonrpsee", log::LevelFilter::Debug),
		)
		.build(log4rs::config::Root::builder().appender("stdout").build(log::LevelFilter::Info))?;

	log4rs::init_config(log_config)?;

	Ok(())
}

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

	async fn some_err() -> Result<String, String> {
		Err("some err".to_string())
	}

	#[tokio::test]
	async fn print_async_doesnt_fail_if_result_is_error() {
		let result = super::print_result(some_err()).await;
		assert!(result.is_ok());
	}
}
