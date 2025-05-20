use crate::{GenesisUtxo, PaymentFilePath};
use partner_chains_cardano_offchain::governed_map::{
	run_get, run_insert, run_list, run_remove, run_update,
};
use serde_json::json;
use sidechain_domain::byte_string::ByteString;
use std::collections::HashMap;

#[derive(Clone, Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
/// Commands for managing the Governed Map key-value store on Cardano
pub enum GovernedMapCmd {
	/// Inserts a key-value pair into the Governed Map. If a value for the key already exists it won't be updated.
	///
	/// NOTE: In rare cases, race conditions may occur, and two inserts with the same key will both succeed.
	/// In that case the second one in terms of block and transaction number is considered valid.
	Insert(InsertCmd),
	/// Updates a key-value pair in the Governed Map. If the key is missing it won't be inserted.
	Update(UpdateCmd),
	/// Removes a key-value pair from the Governed Map
	Remove(RemoveCmd),
	/// Lists all key-value pairs currently stored in the Governed Map
	List(ListCmd),
	/// Retrieves the value stored in the Governed Map for the given key
	Get(GetCmd),
}

impl GovernedMapCmd {
	/// Executes the internal command
	pub async fn execute(self) -> crate::SubCmdResult {
		match self {
			Self::Insert(cmd) => cmd.execute().await,
			Self::Update(cmd) => cmd.execute().await,
			Self::Remove(cmd) => cmd.execute().await,
			Self::List(cmd) => cmd.execute().await,
			Self::Get(cmd) => cmd.execute().await,
		}
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for inserting a key-value pair into the Governed Map
pub struct InsertCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// The key of the entry, UTF-8 encodable string.
	key: String,
	#[arg(long)]
	/// The value of the entry, hex encoded bytes.
	value: ByteString,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl InsertCmd {
	/// Inserts a key-value pair into the Governed Map.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;

		let client = self.common_arguments.get_ogmios_client().await?;

		let result = run_insert(
			self.genesis_utxo.into(),
			self.key,
			self.value,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await;
		print_result_json(result)
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for updating a existing key-value pair in the Governed Map
pub struct UpdateCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// The key of the entry, UTF-8 encodable string.
	key: String,
	#[arg(long)]
	/// The value of the entry, hex encoded bytes.
	value: ByteString,
	#[arg(long)]
	/// If provided, update will fail unless the current value matches the one on the ledger.
	current_value: Option<ByteString>,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl UpdateCmd {
	/// Updates a key-value pair in the Governed Map.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;

		let client = self.common_arguments.get_ogmios_client().await?;

		let result = run_update(
			self.genesis_utxo.into(),
			self.key,
			self.value,
			self.current_value,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await;
		print_result_json(result)
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for removing a key-value pair from the Governed Map
pub struct RemoveCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// The key of the entry, UTF-8 encodable string.
	key: String,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl RemoveCmd {
	/// Removes a key-value pair from the Governed Map.
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;

		let client = self.common_arguments.get_ogmios_client().await?;

		let result = run_remove(
			self.genesis_utxo.into(),
			self.key,
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await;
		print_result_json(result)
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for listing all key-value pairs currently stored in the Governed Map
pub struct ListCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl ListCmd {
	/// Lists all key-value pairs currently stored in the Governed Map.
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;
		let kv_pairs: HashMap<_, _> = run_list(self.genesis_utxo.into(), &client)
			.await?
			.map(|datum| (datum.key, datum.value.to_hex_string()))
			.collect();

		Ok(json!(kv_pairs))
	}
}

#[derive(Clone, Debug, clap::Parser)]
/// Command for retrieving the value stored in the Governed Map for the given key
pub struct GetCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// The key of the entry, UTF-8 encodable string.
	key: String,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl GetCmd {
	/// Retrieves the value stored in the Governed Map for the given key.
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;
		let Some(value) = run_get(self.genesis_utxo.into(), self.key.clone(), &client).await?
		else {
			return Ok(json!({}).into());
		};

		Ok(json!(value.to_hex_string()).into())
	}
}

/// Converts the result of a command into a JSON object.
fn print_result_json(
	result: anyhow::Result<Option<crate::MultiSigSmartContractResult>>,
) -> crate::SubCmdResult {
	match result {
		Err(err) => Err(err)?,
		Ok(Some(res)) => Ok(json!(res)),
		Ok(None) => Ok(json!({})),
	}
}
