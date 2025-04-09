use partner_chains_cardano_offchain::scripts_data::get_scripts_data_with_ogmios;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug, clap::Parser)]
pub struct GetScripts {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	/// Genesis UTXO that identifies the partner chain.
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl GetScripts {
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;
		let scripts_data = get_scripts_data_with_ogmios(self.genesis_utxo, &client).await?;
		Ok(serde_json::json!(scripts_data))
	}
}
