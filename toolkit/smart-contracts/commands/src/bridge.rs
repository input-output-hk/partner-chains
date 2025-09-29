use crate::{GenesisUtxo, PaymentFilePath, transaction_submitted_json};
use partner_chains_cardano_offchain::bridge::{
	create_validator_utxos, deposit_with_ics_spend, deposit_without_ics_input, init_ics_scripts,
};
use sidechain_domain::AssetId;
use sp_runtime::AccountId32;
use std::num::NonZero;

#[derive(Clone, Debug, clap::Subcommand)]
/// Command to initialize and make deposits to the bridge
pub enum BridgeCmd {
	/// Initialize Bridge Smart Conctracts in the Versioning System
	Init(BridgeInitCmd),
	/// Create UTXOs with special tokens at the Bridge Validator. These tokens are used to help with coin selection problem.
	CreateUtxos(BridgeCreateUtxosCmd),
	/// Deposits tokens from payment key wallet to the reserve
	Deposit(BridgeDepositCmd),
}

impl BridgeCmd {
	/// Executes the internal command
	pub async fn execute(self) -> crate::SubCmdResult {
		match self {
			Self::Init(cmd) => cmd.execute().await,
			Self::CreateUtxos(cmd) => cmd.execute().await,
			Self::Deposit(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Initializes bridge smart contracts in the versioning system.
pub struct BridgeInitCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl BridgeInitCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = init_ics_scripts(
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
/// Create UTXOs with special tokens at the Bridge Validator. These tokens are used to help with coin selection problem.
pub struct BridgeCreateUtxosCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
	#[arg(long)]
	/// Number of UTXOs to create
	amount: NonZero<u64>,
}

impl BridgeCreateUtxosCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let result = create_validator_utxos(
			self.genesis_utxo.into(),
			self.amount,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for sending a native token bridge transfer.
pub struct BridgeDepositCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// AssetId of tokens to transfer
	token: AssetId,
	#[arg(long)]
	/// Number of tokens to transfer
	amount: NonZero<u64>,
	#[arg(long)]
	/// Address in the partner chain to transfer the tokens, in hex format.
	pc_address: AccountId32,
	#[arg(long)]
	/// When true, the transaction won't spend an UTXO from ICS Validator. This costs more ada, but is simpler.
	simple: bool,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl BridgeDepositCmd {
	/// Deposits user token in the Bridge.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let tx_hash = if self.simple {
			deposit_without_ics_input(
				self.genesis_utxo.into(),
				self.token,
				self.amount,
				self.pc_address.as_ref(),
				&payment_key,
				&client,
				&self.common_arguments.retries(),
			)
			.await?
		} else {
			deposit_with_ics_spend(
				self.genesis_utxo.into(),
				self.token,
				self.amount,
				self.pc_address.as_ref(),
				&payment_key,
				&client,
				&self.common_arguments.retries(),
			)
			.await?
		};
		Ok(transaction_submitted_json(tx_hash))
	}
}
