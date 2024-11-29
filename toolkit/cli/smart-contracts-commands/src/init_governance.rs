use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::init_governance::run_init_governance;
use sidechain_domain::{MainchainAddressHash, UtxoId};

use crate::read_private_key_from_file;

#[derive(Clone, Debug, clap::Parser)]
pub struct InitGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	governance_authority: MainchainAddressHash,
	#[arg(long)]
	payment_key_file: String,
	#[arg(long)]
	genesis_utxo: Option<UtxoId>,
}

impl InitGovernanceCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = read_private_key_from_file(&self.payment_key_file)?;
		let client = HttpClient::builder().build(self.common_arguments.ogmios_host)?;

		run_init_governance(self.governance_authority, payment_key, self.genesis_utxo, &client)
			.await?;

		Ok(())
	}
}
