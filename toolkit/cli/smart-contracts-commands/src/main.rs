use clap::Parser;
use partner_chains_smart_contracts_commands::SmartContractsCmd;

#[derive(Clone, Debug, clap::Parser)]
pub enum SmartContractsCmdStandalone {
	#[clap(flatten)]
	Inner(SmartContractsCmd),
}

type CmdResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[tokio::main]
async fn main() -> CmdResult<()> {
	let SmartContractsCmdStandalone::Inner(cmd) = SmartContractsCmdStandalone::parse();
	cmd.execute().await
}
