//! Provides wizard style CLI for Partner Chains setup and user operations.
//! Interacts with Smart Contracts using [`partner_chains_cardano_offchain`] crate.
#![deny(missing_docs)]

mod cardano_key;
mod cmd_traits;
mod config;
mod create_chain_spec;
mod data_source;
mod deregister;
mod generate_keys;
mod io;
mod keystore;
mod ogmios;
mod permissioned_candidates;
mod prepare_configuration;
mod register;
mod runtime_bindings;
mod select_utxo;
mod setup_main_chain_state;
mod start_node;

#[cfg(test)]
mod tests;

use clap::Parser;
pub use create_chain_spec::CreateChainSpecConfig;
pub use io::DefaultCmdRunContext;
use io::*;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
pub use permissioned_candidates::{ParsedPermissionedCandidatesKeys, PermissionedCandidateKeys};
pub use runtime_bindings::{PartnerChainRuntime, RuntimeTypeWrapper};
use std::time::Duration;

#[derive(Clone, Debug, clap::Parser)]
#[command(author, version, about, long_about = None)]
pub(crate) struct CommonArguments {
	#[arg(default_value = "5", long)]
	retry_delay_seconds: u64,
	#[arg(default_value = "59", long)]
	retry_count: usize,
}

impl CommonArguments {
	pub(crate) fn retries(&self) -> FixedDelayRetries {
		FixedDelayRetries::new(Duration::from_secs(self.retry_delay_seconds), self.retry_count)
	}
}

#[derive(Clone, Debug, Parser)]
#[command(
    after_long_help = HELP_EXAMPLES,
)]
/// Partner Chains text "wizards" for setting up a chain.
pub enum Command<T: PartnerChainRuntime> {
	/// This wizard generates the keys required for operating a partner-chains node, stores them in the keystore directory, and prints the public keys and keystore location.
	GenerateKeys(generate_keys::GenerateKeysCmd),
	/// Wizard to obtain the configuration needed for the partner-chain governance authority. This configuration should be shared with chain participants and used to create the chain spec json file.
	PrepareConfiguration(prepare_configuration::PrepareConfigurationCmd),
	/// Wizard for setting D-parameter and Permissioned Candidates list on the main chain.
	/// Uses 'chain config' obtained after running `prepare-configuration`.
	SetupMainChainState(setup_main_chain_state::SetupMainChainStateCmd<T>),
	/// Wizard for creating a chain spec json file based on the chain configuration (see `prepare-configuration`).
	CreateChainSpec(create_chain_spec::CreateChainSpecCmd<T>),
	/// Wizard for starting a substrate node in the environment set up by `generate-keys`,
	/// `prepare-config`, and `create-chain-spec`. It also assists in setting the `resources configuration`.
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

impl<T: PartnerChainRuntime> Command<T> {
	/// Runs a Partner Chain wizard command.
	pub fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()> {
		match self {
			Command::GenerateKeys(cmd) => cmd.run(context),
			Command::PrepareConfiguration(cmd) => cmd.run(context),
			Command::CreateChainSpec(cmd) => cmd.run(context),
			Command::SetupMainChainState(cmd) => cmd.run(context),
			Command::StartNode(cmd) => cmd.run(context),
			Command::Register1(cmd) => cmd.run(context),
			Command::Register2(cmd) => cmd.run(context),
			Command::Register3(cmd) => cmd.run(context),
			Command::Deregister(cmd) => cmd.run(context),
		}
	}
}

/// Trait representing a runnable command.
pub trait CmdRun {
	/// Runs command. The implemented command should do IO through the passed in `context` object.
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()>;
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
║   3. setup-main-chain-state: configure the main chain parameters               ║
║   4. create-chain-spec     : create the chain specification file               ║
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
║         pc-chain-config.json files have been obtained from                     ║
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
║         chain-spec.json and pc-chain-config.json files have                    ║
║         been obtained from the Governance Authority and are present in the     ║
║         working directory.                                                     ║
╚════════════════════════════════════════════════════════════════════════════════╝
"#;
