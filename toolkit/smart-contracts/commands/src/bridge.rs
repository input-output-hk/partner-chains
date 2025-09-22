use crate::{GenesisUtxo, PaymentFilePath, transaction_submitted_json};
use partner_chains_cardano_offchain::bridge::{deposit_with_ics_spend, deposit_without_ics_input};
use sp_runtime::AccountId32;

#[derive(Clone, Debug, clap::Parser)]
/// Command for sending a native token bridge transfer. Token to be sent is defined in the Reserve Settings.
pub struct BridgeCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// Number of tokens to transfer
	amount: u64,
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

impl BridgeCmd {
	/// Creates the D-parameter and upserts it on the main chain.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;
		let client = self.common_arguments.get_ogmios_client().await?;
		let tx_hash = if self.simple {
			deposit_without_ics_input(
				self.genesis_utxo.into(),
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
