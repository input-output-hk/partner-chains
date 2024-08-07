use clap::Parser;
use sc_cli::{CliConfiguration, SharedParams};
use sidechain_domain::{MainchainPublicKey, McEpochNumber};

#[derive(Debug, Clone, Parser)]
pub struct RegistrationStatusCmd {
	#[arg(long)]
	pub mainchain_pub_key: MainchainPublicKey,
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
