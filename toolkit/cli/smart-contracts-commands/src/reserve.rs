use crate::PaymentFilePath;
use ogmios_client::jsonrpsee::client_for_url;
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries,
	reserve::{
		create::{create_reserve_utxo, ReserveParameters},
		deposit::{deposit_to_reserve, TokenAmount},
		init::init_reserve_management,
		update_settings::update_reserve_settings,
	},
};
use sidechain_domain::{AssetId, ScriptHash, UtxoId};

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum ReserveCmd {
	/// Initialize the reserve management system for your chain
	Init(InitReserveCmd),
	Create(CreateReserveCmd),
	Deposit(DepositReserveCmd),
	/// Update reserve management system settings for your chain
	UpdateSettings(UpdateReserveSettingsCmd),
}

impl ReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		match self {
			Self::Init(cmd) => cmd.execute().await,
			Self::Create(cmd) => cmd.execute().await,
			Self::Deposit(cmd) => cmd.execute().await,
			Self::UpdateSettings(cmd) => cmd.execute().await,
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
		let ogmios_client = client_for_url(&self.common_arguments.ogmios_url).await?;
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
	/// Incentive amount of tokens.
	#[arg(long, default_value = "0")]
	initial_incentive_amount: u64,
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
		let ogmios_client = client_for_url(&self.common_arguments.ogmios_url).await?;
		let _ = create_reserve_utxo(
			ReserveParameters {
				initial_incentive: self.initial_incentive_amount,
				total_accrued_function_script_hash: self.total_accrued_function_script_hash,
				token: self.token,
				initial_deposit: self.initial_deposit_amount,
			},
			self.genesis_utxo,
			payment_key.0,
			&ogmios_client,
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
	/// Genesis UTXO of the partner-chain.
	#[arg(long, short('c'))]
	genesis_utxo: UtxoId,
	/// Reserve token asset id encoded in form <policy_id_hex>.<asset_name_hex>.
	#[arg(long)]
	token: AssetId,
	/// Amount of tokens to deposit. They must be present in the payment wallet.
	#[arg(long)]
	amount: u64,
}

impl DepositReserveCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let ogmios_client = client_for_url(&self.common_arguments.ogmios_url).await?;
		let _ = deposit_to_reserve(
			TokenAmount { token: self.token, amount: self.amount },
			self.genesis_utxo,
			payment_key.0,
			&ogmios_client,
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
	total_accrued_function_script_hash: Option<ScriptHash>,
	#[arg(long)]
	/// Incentive amount of tokens.
	reserve_initial_incentive_amount: Option<u64>,
}

impl UpdateReserveSettingsCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;
		let ogmios_client = client_for_url(&self.common_arguments.ogmios_url).await?;
		let _ = update_reserve_settings(
			self.genesis_utxo,
			payment_key.0,
			self.total_accrued_function_script_hash,
			self.reserve_initial_incentive_amount,
			&ogmios_client,
			&FixedDelayRetries::two_minutes(),
		)
		.await?;
		Ok(())
	}
}
