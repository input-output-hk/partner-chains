use jsonrpsee::http_client::HttpClient;
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries, reserve::init::init_reserve_management,
};
use sidechain_domain::UtxoId;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ReserveCmd {
	/// Initialize the reserve management system for your chain
	Init(InitReserveCmd),
}

impl ReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		match self {
			Self::Init(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct InitReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long, short('k'))]
	payment_key_file: String,
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl InitReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = crate::read_private_key_from_file(&self.payment_key_file)?;
		let ogmios_client = HttpClient::builder().build(self.common_arguments.ogmios_url)?;
		let _ = init_reserve_management(
			self.genesis_utxo,
			payment_key.0,
			&ogmios_client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}
