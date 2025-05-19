use crate::{GenesisUtxo, PaymentFilePath, option_to_json};
use partner_chains_cardano_offchain::d_param::upsert_d_param;
use sidechain_domain::DParameter;

#[derive(Clone, Debug, clap::Parser)]
/// Command for upserting the D-parameter on the main chain
pub struct UpsertDParameterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// Number of permissioned candidates
	permissioned_candidates_count: u16,
	#[arg(long)]
	/// Number of registered candidates
	registered_candidates_count: u16,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl UpsertDParameterCmd {
	/// Creates the D-parameter and upserts it on the main chain.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let d_param = DParameter {
			num_permissioned_candidates: self.permissioned_candidates_count,
			num_registered_candidates: self.registered_candidates_count,
		};
		let client = self.common_arguments.get_ogmios_client().await?;

		let result = upsert_d_param(
			self.genesis_utxo.into(),
			&d_param,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;

		Ok(option_to_json(result))
	}
}
