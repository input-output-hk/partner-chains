use crate::PaymentFilePath;
use partner_chains_cardano_offchain::sign_tx::sign_tx;
use serde_json::json;
use sidechain_domain::TransactionCbor;

#[derive(Clone, Debug, clap::Parser)]
pub struct SignTxCmd {
	#[arg(long, value_parser = TransactionCbor::decode_hex)]
	/// Hex-encoded transaction CBOR (with or without 0x prefix)
	transaction: TransactionCbor,
	#[clap(flatten)]
	payment_key_file: PaymentFilePath,
}

impl SignTxCmd {
	pub async fn execute(self) -> crate::CmdResult<()> {
		let payment_key = self.payment_key_file.read_key()?;

		let vkey_witness = sign_tx(self.transaction.0, &payment_key)?;

		let json = json!(
			{
				"type": "TxWitness ConwayEra",
				"description": "",
				"cborHex": hex::encode(vkey_witness)
			}
		);

		println!("{}", json);

		Ok(())
	}
}
