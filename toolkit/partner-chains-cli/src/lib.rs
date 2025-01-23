//! Provides wizard style CLI for Partner Chains setup and user operations.
//! Interacts with Smart Contracts using [`partner_chains_cardano_offchain`] crate.

pub(crate) mod cardano_key;
pub mod config;
pub mod create_chain_spec;
mod deregister;
pub mod generate_keys;
pub mod io;
pub mod keystore;
pub(crate) mod main_chain_follower;
pub(crate) mod ogmios;
pub(crate) mod permissioned_candidates;
mod prepare_configuration;
pub mod register;
pub(crate) mod select_utxo;
mod setup_main_chain_state;
pub mod start_node;

#[cfg(test)]
mod tests;

use clap::Parser;
use io::*;
use log4rs::{
	append::{console::ConsoleAppender, file::FileAppender},
	config::Appender,
};

#[derive(Clone, Debug, Parser)]
#[command(
    after_long_help = HELP_EXAMPLES,
)]
pub enum Command {
	/// This wizard generates the keys required for operating a partner-chains node, stores them in the keystore directory, and prints the public keys and keystore location.
	GenerateKeys(generate_keys::GenerateKeysCmd),
	/// Wizard to obtain the configuration needed for the partner-chain governance authority. This configuration should be shared with chain participants and used to create the chain spec json file.
	PrepareConfiguration(prepare_configuration::PrepareConfigurationCmd),
	/// Wizard for creating a chain spec json file based on the chain configuration (see `prepare-configuration`).
	CreateChainSpec(create_chain_spec::CreateChainSpecCmd),
	/// Wizard for setting D-parameter and Permissioned Candidates list on the main chain.
	/// Uses 'chain config' obtained after running `prepare-configuration`.
	SetupMainChainState(setup_main_chain_state::SetupMainChainStateCmd),
	/// Wizard for starting a substrate node in the environment set up by `generate-keys`,
	/// `prepare-config`, and `create-chain-spec`. It also assits in setting the `resources configuration`.
	StartNode(start_node::StartNodeCmd),
	/// The first step of registering as a committee candidate. Registration is split into three steps to allow the user to use their cold keys on a cold machine.
	Register1(register::register1::Register1Cmd),
	/// The second step of registering as a committee candidate, using cold keys.
	Register2(register::register2::Register2Cmd),
	/// The final step of registering as a committee candidate, not using cold keys.
	Register3(register::register3::Register3Cmd),
	/// Deregister from the candidates set. This command requires chain config file present in the running directory.
	Deregister(deregister::DeregisterCmd),
}

pub trait CmdRun {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()>;
}

pub fn run(command: Command) -> anyhow::Result<()> {
	let log_config = log4rs::config::Config::builder()
		.appender(Appender::builder().build("stdout", Box::new(ConsoleAppender::builder().build())))
		.appender(
			Appender::builder()
				.build("ogmios-log", Box::new(FileAppender::builder().build("ogmios_client.log")?)),
		)
		.logger(
			log4rs::config::Logger::builder()
				.appender("ogmios-log")
				.additive(false)
				.build("ogmios_client::jsonrpsee", log::LevelFilter::Debug),
		)
		.build(log4rs::config::Root::builder().appender("stdout").build(log::LevelFilter::Info))?;
	log4rs::init_config(log_config)?;
	match command {
		Command::GenerateKeys(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::PrepareConfiguration(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::CreateChainSpec(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::SetupMainChainState(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::StartNode(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::Register1(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::Register2(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::Register3(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::Deregister(cmd) => cmd.run(&DefaultCmdRunContext)?,
	}
	Ok(())
}

const HELP_EXAMPLES: &str = r#"
╔════════════════════════════════════════════════════════════════════════════════╗
║                           Command groups by role                               ║
╠════════════════════════════════════════════════════════════════════════════════╣
║ The following sections outline the typical sequence of commands for each role. ║
║ The numbering indicates the recommended order of execution. Please note that   ║
║ this order may vary depending on specific deployment scenarios.                ║
╟────────────────────────────────────────────────────────────────────────────────╢
║ Governance Authority:                                                          ║
║   1. generate-keys         : generate necessary cryptographic keys             ║
║   2. prepare-configuration : set up the partner chain configuration            ║
║   3. create-chain-spec     : create the chain specification file               ║
║   4. setup-main-chain-state: configure the main chain parameters               ║
║   5. start-node            : start the validator node                          ║
╟────────────────────────────────────────────────────────────────────────────────╢
║ Registered Validator:                                                          ║
║   1. generate-keys         : generate validator keys                           ║
║   2. register1             : initiate the registration process                 ║
║   3. register2             : complete registration with cold keys              ║
║   4. register3             : finalize registration                             ║
║   5. start-node            : start the validator node                          ║
║   6. deregister            : cancel registration                               ║
║                                                                                ║
║   Note: This sequence assumes that the chain-spec.json and                     ║
║         partner-chains-cli-chain-config.json files have been obtained from     ║
║         the Governance Authority and are present in the working directory.     ║
╟────────────────────────────────────────────────────────────────────────────────╢
║ Permissioned Validator:                                                        ║
║   1. generate-keys         : generate validator keys                           ║
║   2. start-node            : start the validator node                          ║
║                                                                                ║
║   Note: After executing 'generate-keys', the generated keys must be shared     ║
║         with the Governance Authority. The 'start-node' command can only be    ║
║         executed after the Governance Authority has established the partner    ║
║         chain on the main network. This sequence assumes that the              ║
║         chain-spec.json and partner-chains-cli-chain-config.json files have    ║
║         been obtained from the Governance Authority and are present in the     ║
║         working directory.                                                     ║
╚════════════════════════════════════════════════════════════════════════════════╝
"#;
