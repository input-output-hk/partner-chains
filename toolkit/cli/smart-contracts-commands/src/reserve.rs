use crate::PaymentFilePath;
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries,
	reserve::{
		create::{create_reserve_utxo, ReserveParameters},
		deposit::deposit_to_reserve,
		handover::handover_reserve,
		init::init_reserve_management,
		release::release_reserve_funds,
		update_settings::update_reserve_settings,
	},
};
use sidechain_domain::{AssetId, ScriptHash, UtxoId};
use std::num::NonZero;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ReserveCmd {
	/// Initialize the reserve management system for your chain
	Init(InitReserveCmd),
	/// Creates the reserve for your chain. `init` and `create` will be merged into a single command in a future release.
	Create(CreateReserveCmd),
	/// Deposits tokens from payment key wallet to the reserve
	Deposit(DepositReserveCmd),
	/// Update reserve management system settings for your chain
	UpdateSettings(UpdateReserveSettingsCmd),
	/// Releases all the remaining funds from the reserve to the illiquid supply
	Handover(HandoverReserveCmd),
	/// Releases funds from the reserve to the illiquid supply
	Release(ReleaseReserveCmd),
}

impl ReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		match self {
			Self::Init(cmd) => cmd.execute().await,
			Self::Create(cmd) => cmd.execute().await,
			Self::Deposit(cmd) => cmd.execute().await,
			Self::UpdateSettings(cmd) => cmd.execute().await,
			Self::Handover(cmd) => cmd.execute().await,
			Self::Release(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct InitReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the partner-chain.
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl InitReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let _ = init_reserve_management(
			self.genesis_utxo,
			&payment_key,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct CreateReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the partner-chain.
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
	/// Script hash of the 'total accrued function', also called V-function, that computes how many tokens could be released from the reserve at given moment.
	#[arg(long)]
	total_accrued_function_script_hash: ScriptHash,
	/// Initial amount of tokens to deposit. They must be present in the payment wallet.
	#[arg(long)]
	initial_deposit_amount: u64,
	/// Reserve token asset id encoded in form <policy_id_hex>.<asset_name_hex>.
	#[arg(long)]
	token: AssetId,
}

impl CreateReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let _ = create_reserve_utxo(
			ReserveParameters {
				total_accrued_function_script_hash: self.total_accrued_function_script_hash,
				token: self.token,
				initial_deposit: self.initial_deposit_amount,
			},
			self.genesis_utxo,
			&payment_key,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct DepositReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the partner-chain, identifies the partner chain and its reserve.
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
	/// Amount of tokens to deposit. They must be present in the payment wallet.
	#[arg(long)]
	amount: u64,
}

impl DepositReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let _ = deposit_to_reserve(
			self.amount,
			self.genesis_utxo,
			&payment_key,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct UpdateReserveSettingsCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the partner-chain.
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
	#[arg(long)]
	/// Script hash of the 'total accrued function', also called V-function, that computes how many tokens could be released from the reserve at given moment.
	total_accrued_function_script_hash: ScriptHash,
}

impl UpdateReserveSettingsCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let _ = update_reserve_settings(
			self.genesis_utxo,
			&payment_key,
			self.total_accrued_function_script_hash,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct HandoverReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the partner-chain.
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
}

impl HandoverReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let _ = handover_reserve(
			self.genesis_utxo,
			&payment_key,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct ReleaseReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	/// Genesis UTXO of the partner-chain.
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
	/// Reference UTXO containing the V-Function script
	#[arg(long, short('r'))]
	reference_utxo: UtxoId,
	/// Amount of reserve tokens to be released to the illiquid supply.
	#[arg(long)]
	amount: NonZero<u64>,
}

impl ReleaseReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let _ = release_reserve_funds(
			self.amount,
			self.genesis_utxo,
			self.reference_utxo,
			&payment_key,
			&client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}
