use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::scripts_data::get_scripts_data_with_ogmios;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug, clap::Parser)]
pub struct GetScripts {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long, short = 'c')]
	genesis_utxo: UtxoId,
}

impl GetScripts {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let client = HttpClient::builder().build(self.common_arguments.ogmios_host)?;
		let scripts_data = get_scripts_data_with_ogmios(self.genesis_utxo, client).await?;

		let json = serde_json::to_string_pretty(&scripts_data)?;

		print!("{json}");

		Ok(())
	}
}
