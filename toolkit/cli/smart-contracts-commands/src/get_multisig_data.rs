use sidechain_domain::UtxoId;

#[derive(Clone, Debug, clap::Parser)]
pub struct GetGovernancePolicy {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Genesis UTXO that identifies the partner chain.
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl GetGovernancePolicy {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let client = self.common_arguments.get_ogmios_client().await?;
		let summary = partner_chains_cardano_offchain::governance::get_governance_policy_summary(
			self.genesis_utxo,
			&client,
		)
		.await?;
		print!("{}", serde_json::to_string(&summary)?);
		Ok(())
	}
}
