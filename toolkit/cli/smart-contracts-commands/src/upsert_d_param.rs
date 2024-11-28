use crate::read_private_key_from_file;
use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::d_param::upsert_d_param;
use sidechain_domain::{DParameter, UtxoId};

#[derive(Clone, Debug, clap::Parser)]
pub struct UpsertDParam {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	payment_key_file: String,
	#[arg(long)]
	genesis_utxo: UtxoId,
	/// Number of committee seats reserved for permissioned candidates
	#[arg(long)]
	permissioned: u16,
	/// Number of committee seats reserved for registered candidates
	#[arg(long)]
	registered: u16,
}

impl UpsertDParam {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = read_private_key_from_file(&self.payment_key_file)?;
		let client = HttpClient::builder().build(self.common_arguments.ogmios_host)?;

		let d_param = DParameter {
			num_permissioned_candidates: self.permissioned,
			num_registered_candidates: self.registered,
		};

		let _ = upsert_d_param(self.genesis_utxo, &d_param, payment_key.0, &client).await?;

		Ok(())
	}
}
