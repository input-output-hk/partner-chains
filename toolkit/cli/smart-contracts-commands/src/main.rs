use clap::Parser;
use partner_chains_smart_contracts_commands::{setup_logging, SmartContractsCmd};

#[derive(Clone, Debug, clap::Parser)]
pub enum SmartContractsCmdStandalone {
	#[clap(flatten)]
	Inner(SmartContractsCmd),
}

type CmdResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> CmdResult<()> {
	setup_logging()?;
	let SmartContractsCmdStandalone::Inner(cmd) = SmartContractsCmdStandalone::parse();
	cmd.execute().await
}
