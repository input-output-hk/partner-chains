use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::csl::HelperTransaction;
use partner_chains_cardano_offchain::csl::HelperVKeyWitness;
use partner_chains_cardano_offchain::assemble_tx::assemble_tx;
use sidechain_domain::MainchainSignature;

#[derive(Clone, Debug, clap::Parser)]
pub struct AssembleTxCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	transaction: String,
    #[arg(short, long, num_args = 1.., value_delimiter = ' ')]
    witnesses: Vec<String>,
}

impl AssembleTxCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {

		let client = self.common_arguments.get_ogmios_client().await?;

        let transaction = HelperTransaction::new(hex::decode(self.transaction)?).to_csl()?;

        let mut witnesses = vec![];

        for w in self.witnesses.iter() {
            witnesses.push(HelperVKeyWitness::new(hex::decode(w)?).to_csl()?);
        }

        assemble_tx(transaction, witnesses, &client, &FixedDelayRetries::two_minutes()).await?;

		Ok(())
	}
}
