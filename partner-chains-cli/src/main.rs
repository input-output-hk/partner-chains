mod cardano_cli;
pub mod config;
pub mod create_chain_spec;
pub mod generate_keys;
pub mod io;
pub mod keystore;
pub(crate) mod main_chain_follower;
pub(crate) mod permissioned_candidates;
mod prepare_configuration;
pub mod register;
mod setup_main_chain_state;
pub(crate) mod pc_contracts_cli_resources;
pub(crate) mod smart_contracts;
pub mod start_node;

#[cfg(test)]
mod tests;

use clap::Parser;
use io::*;

#[derive(Debug, Parser)]
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
}

pub trait CmdRun {
	fn run<C: IOContext>(&self, context: &C) -> anyhow::Result<()>;
}

fn main() -> anyhow::Result<()> {
	let args = Command::parse();
	match args {
		Command::GenerateKeys(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::PrepareConfiguration(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::CreateChainSpec(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::SetupMainChainState(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::StartNode(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::Register1(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::Register2(cmd) => cmd.run(&DefaultCmdRunContext)?,
		Command::Register3(cmd) => cmd.run(&DefaultCmdRunContext)?,
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
