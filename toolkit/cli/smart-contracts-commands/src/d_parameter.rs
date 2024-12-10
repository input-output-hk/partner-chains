use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::d_param::upsert_d_param;
use sidechain_domain::DParameter;
use sidechain_domain::UtxoId;

use crate::read_private_key_from_file;

#[derive(Clone, Debug, clap::Parser)]
pub struct UpsertDParameterCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	permissioned_candidates_count: u16,
	#[arg(long)]
	registered_candidates_count: u16,
	#[arg(long, short('k'))]
	payment_key_file: String,
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl UpsertDParameterCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = read_private_key_from_file(&self.payment_key_file)?;
		let d_param = DParameter {
			num_permissioned_candidates: self.permissioned_candidates_count,
			num_registered_candidates: self.registered_candidates_count,
		};
		let client = HttpClient::builder().build(self.common_arguments.ogmios_url)?;

		upsert_d_param(self.genesis_utxo, &d_param, payment_key.0, &client).await?;

		Ok(())
	}
}
