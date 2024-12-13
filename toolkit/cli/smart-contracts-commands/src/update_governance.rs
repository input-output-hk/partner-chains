use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries, update_governance::run_update_governance,
};
use sidechain_domain::{MainchainAddressHash, UtxoId};

use crate::read_private_key_from_file;

#[derive(Clone, Debug, clap::Parser)]
pub struct UpdateGovernanceCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Governance authority hash to be set.
	#[arg(long, short = 'g')]
	new_governance_authority: MainchainAddressHash,
	/// Path to the Cardano Payment Key file.
	#[arg(long, short = 'k')]
	payment_key_file: String,
	/// Genesis UTXO of the new chain, it will be spent durning initialization. If not set, then one will be selected from UTXOs of the payment key.
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl UpdateGovernanceCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = read_private_key_from_file(&self.payment_key_file)?;
		let client = HttpClient::builder().build(self.common_arguments.ogmios_url)?;

		run_update_governance(
			self.new_governance_authority,
			payment_key,
			self.genesis_utxo,
			&client,
			FixedDelayRetries::two_minutes(),
		)
		.await?;

		Ok(())
	}
}
