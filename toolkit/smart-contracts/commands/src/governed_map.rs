use crate::PaymentFilePath;
use partner_chains_cardano_offchain::await_tx::FixedDelayRetries;
use partner_chains_cardano_offchain::governed_map::run_insert;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::UtxoId;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
pub enum GovernedMapCmd {
	/// Inserts a key-value pair into the Governed Gap. If a value for the key already exists it won't be updated.
	///
	/// NOTE: In rare cases, race conditions may occur, and two inserts with the same key will both succeed.
	/// In that case the second one in terms of block and transaction number is considered valid.
	Insert(InsertCmd),
}

impl GovernedMapCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		match self {
			Self::Insert(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
pub struct InsertCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	key: String,
	#[arg(long)]
	value: ByteString,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
	#[arg(long, short('g'))]
	genesis_utxo: UtxoId,
}

impl InsertCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;

		let client = self.common_arguments.get_ogmios_client().await?;

		let result = run_insert(
			self.genesis_utxo,
			self.key,
			self.value,
			&payment_key,
			&client,
			&FixedDelayRetries::five_minutes(),
		)
		.await?;
		Ok(serde_json::json!(result))
	}
}
