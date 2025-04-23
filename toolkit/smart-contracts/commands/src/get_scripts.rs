use crate::GenesisUtxo;
use partner_chains_cardano_offchain::scripts_data::get_scripts_data_with_ogmios;

#[derive(Clone, Debug, clap::Parser)]
pub struct GetScripts {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	genesis_utxo: GenesisUtxo,
}

impl GetScripts {
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;
		let scripts_data = get_scripts_data_with_ogmios(self.genesis_utxo.into(), &client).await?;
		Ok(serde_json::json!(scripts_data))
	}
}
