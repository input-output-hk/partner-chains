use crate::{option_to_json, PaymentFilePath};
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::d_param::upsert_d_param;
use sidechain_domain::{DParameter, UtxoId};

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
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl UpsertDParameterCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let d_param = DParameter {
			num_permissioned_candidates: self.permissioned_candidates_count,
			num_registered_candidates: self.registered_candidates_count,
		};
		let client = self.common_arguments.get_ogmios_client().await?;

		let result = upsert_d_param(
			self.genesis_utxo,
			&d_param,
			&payment_key,
			&client,
			&FixedDelayRetries::five_minutes(),
		)
		.await?;

		Ok(option_to_json(result))
	}
}
