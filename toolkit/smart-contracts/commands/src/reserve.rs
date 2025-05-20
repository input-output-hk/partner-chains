use crate::{GenesisUtxo, PaymentFilePath, option_to_json, transaction_submitted_json};
use partner_chains_cardano_offchain::reserve::{
	create::{ReserveParameters, create_reserve_utxo},
	deposit::deposit_to_reserve,
	handover::handover_reserve,
	init::init_reserve_management,
	release::release_reserve_funds,
	update_settings::update_reserve_settings,
};
use sidechain_domain::{AssetId, ScriptHash, UtxoId};
use std::num::NonZero;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
/// Command for managing the reserve on the main chain
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
	/// Executes the internal command
	pub async fn execute(self) -> crate::SubCmdResult {
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
/// Command for initializing the components neccesary for operation of the reserve management system for your chain
pub struct InitReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl InitReserveCmd {
	/// Initializes the components neccesary for operation of the reserve management system for your chain
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = init_reserve_management(
			self.genesis_utxo.into(),
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for creating the reserve for your chain
pub struct CreateReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
	#[arg(long)]
	/// Script hash of the 'total accrued function', also called V-function, that computes how many tokens could be released from the reserve at given moment.
	total_accrued_function_script_hash: ScriptHash,
	#[arg(long)]
	/// Initial amount of tokens to deposit. They must be present in the payment wallet.
	initial_deposit_amount: u64,
	#[arg(long)]
	/// Reserve token asset id encoded in form <policy_id_hex>.<asset_name_hex>.
	token: AssetId,
}

impl CreateReserveCmd {
	/// Creates the reserve for your chain
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = create_reserve_utxo(
			ReserveParameters {
				total_accrued_function_script_hash: self.total_accrued_function_script_hash,
				token: self.token,
				initial_deposit: self.initial_deposit_amount,
			},
			self.genesis_utxo.into(),
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for depositing more tokens to an already existing reserve
pub struct DepositReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
	#[arg(long)]
	/// Amount of reserve tokens to deposit. They must be present in the payment wallet.
	amount: u64,
}

impl DepositReserveCmd {
	/// Deposits tokens from payment key wallet to the reserve
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = deposit_to_reserve(
			self.amount,
			self.genesis_utxo.into(),
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for updating the reserve management system settings for your chain
pub struct UpdateReserveSettingsCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[arg(long, short('c'))]
	/// Genesis UTXO of the partner-chain.
	genesis_utxo: UtxoId,
	#[arg(long)]
	/// Script hash of the 'total accrued function', also called V-function, that computes how many tokens could be released from the reserve at given moment.
	total_accrued_function_script_hash: ScriptHash,
}

impl UpdateReserveSettingsCmd {
	/// Updates the reserve management system settings for your chain
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = update_reserve_settings(
			self.genesis_utxo,
			&payment_key,
			self.total_accrued_function_script_hash,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(option_to_json(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for handing over the remaining funds from the reserve to the illiquid supply.
/// This operation ends the lifecycle of the reserve.
pub struct HandoverReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[arg(long, short('c'))]
	/// Genesis UTXO of the partner-chain.
	genesis_utxo: UtxoId,
}

impl HandoverReserveCmd {
	/// Hands over the remaining funds from the reserve to the illiquid supply.
	/// This operation ends the lifecycle of the reserve.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = handover_reserve(
			self.genesis_utxo,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for releasing funds from the reserve to the illiquid supply
pub struct ReleaseReserveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[arg(long, short('c'))]
	/// Genesis UTXO of the partner-chain.
	genesis_utxo: UtxoId,
	#[arg(long, short('r'))]
	/// Reference UTXO containing the V-Function script
	reference_utxo: UtxoId,
	#[arg(long)]
	/// Amount of reserve tokens to be released to the illiquid supply.
	amount: NonZero<u64>,
}

impl ReleaseReserveCmd {
	/// Releases funds from the reserve to the illiquid supply
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = release_reserve_funds(
			self.amount,
			self.genesis_utxo,
			self.reference_utxo,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(transaction_submitted_json(result))
	}
}
