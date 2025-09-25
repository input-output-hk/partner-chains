use crate::{GenesisUtxo, PaymentFilePath, option_to_json};
use partner_chains_cardano_offchain::{
	plutus_script::PlutusScript, versioning_system::upsert_script,
};
use sidechain_domain::PlutusScriptCbor;

#[derive(Clone, Debug, clap::Parser)]
/// Command for upserting the versioned script on the mainchain
pub struct UpsertScriptCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long)]
	/// Script ID
	script_id: u32,
	#[arg(long)]
	/// CBOR encoded Plutus V2 script
	plutus_script: PlutusScriptCbor,
	#[clap(flatten)]
	/// Path to the payment key file
	payment_key_file: PaymentFilePath,
	#[clap(flatten)]
	/// Genesis UTXO
	genesis_utxo: GenesisUtxo,
}

impl UpsertScriptCmd {
	/// Creates or updates a versioning utxo with reference script
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;

		let client = self.common_arguments.get_ogmios_client().await?;

		let result = upsert_script(
			PlutusScript::v2_from_cbor(&self.plutus_script.0)?,
			self.script_id,
			self.genesis_utxo.into(),
			&payment_key,
			&client,
			&self.common_arguments.retries(),
		)
		.await?;

		Ok(option_to_json(result))
	}
}
