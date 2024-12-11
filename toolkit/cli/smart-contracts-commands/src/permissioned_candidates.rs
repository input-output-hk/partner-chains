use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::permissioned_candidates::upsert_permissioned_candidates;
use sidechain_domain::PermissionedCandidateData;
use sidechain_domain::UtxoId;

use crate::read_private_key_from_file;

#[derive(Clone, Debug, clap::Parser)]
pub struct UpsertPermissionedCandidatesCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	permissioned_candidates_file: String,
	#[arg(long, short('k'))]
	payment_key_file: String,
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl UpsertPermissionedCandidatesCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = read_private_key_from_file(&self.payment_key_file)?;

		let permissioned_candidates: Vec<PermissionedCandidateData> = serde_json::from_str(&std::fs::read_to_string(&self.permissioned_candidates_file)?)?;

		let client = HttpClient::builder().build(self.common_arguments.ogmios_url)?;

		upsert_permissioned_candidates(self.genesis_utxo, &permissioned_candidates, payment_key.0, &client).await?;

		Ok(())
	}
}
