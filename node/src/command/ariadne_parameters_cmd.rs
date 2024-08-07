use clap::Parser;
use sc_cli::{CliConfiguration, SharedParams};
use sidechain_domain::McEpochNumber;

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
