use crate::GenesisUtxo;
use partner_chains_cardano_offchain::scripts_data::get_scripts_data_with_ogmios;

#[derive(Clone, Debug, clap::Parser)]
/// Command for getting the addresses and policy ids of the partner chain smart contracts parametrised with the given genesis utxo
pub struct GetScripts {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl GetScripts {
	/// Gets the addresses and policy ids of the partner chain smart contracts parametrised with the given genesis utxo
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;
		let scripts_data = get_scripts_data_with_ogmios(self.genesis_utxo.into(), &client).await?;
		Ok(serde_json::json!(scripts_data))
	}
}
