use crate::PaymentFilePath;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::d_param::{get_upsert_d_param_tx, upsert_d_param};
use sidechain_domain::DParameter;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum DParameterCmd {
	GetUpsertTransaction(GetUpsertTransaction),
}

impl DParameterCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		match self {
			Self::GetUpsertTransaction(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct GetUpsertTransaction {
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

impl GetUpsertTransaction {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let d_param = DParameter {
			num_permissioned_candidates: self.permissioned_candidates_count,
			num_registered_candidates: self.registered_candidates_count,
		};
		let client = self.common_arguments.get_ogmios_client().await?;

		let tx = get_upsert_d_param_tx(
			self.genesis_utxo,
			&d_param,
			&payment_key,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;

		let tx_hex = tx.to_hex();
		println!("Tx: {tx_hex}");

		let body = tx.body().to_hex();
		println!("Body: {body}");
		Ok(())
	}
}

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
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let d_param = DParameter {
			num_permissioned_candidates: self.permissioned_candidates_count,
			num_registered_candidates: self.registered_candidates_count,
		};
		let client = self.common_arguments.get_ogmios_client().await?;

		upsert_d_param(
			self.genesis_utxo,
			&d_param,
			&payment_key,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;

		Ok(())
	}
}
