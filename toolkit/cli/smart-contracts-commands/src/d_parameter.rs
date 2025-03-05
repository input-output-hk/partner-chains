use crate::PaymentFilePath;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::d_param::upsert_d_param;
use sidechain_domain::DParameter;
use sidechain_domain::MainchainKeyHash;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug, clap::Parser)]
pub struct UpsertDParameterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	permissioned_candidates_count: u16,
	#[arg(long)]
	registered_candidates_count: u16,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	#[arg(short, long, num_args = 1.., value_delimiter = ' ')]
	governance_authority: Vec<MainchainKeyHash>,
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl UpsertDParameterCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let d_param = DParameter {
			num_permissioned_candidates: self.permissioned_candidates_count,
			num_registered_candidates: self.registered_candidates_count,
		};
		let client = self.common_arguments.get_ogmios_client().await?;

		let governance_authority = if self.governance_authority.len() == 0 {
			vec![payment_key.to_pub_key_hash()]
		} else {
			self.governance_authority.clone()
		};

		upsert_d_param(
			self.genesis_utxo,
			&d_param,
			&payment_key,
			governance_authority,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;

		Ok(())
	}
}
