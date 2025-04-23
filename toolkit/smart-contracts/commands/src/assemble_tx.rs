use crate::transaction_submitted_json;
use partner_chains_cardano_offchain::assemble_tx::assemble_tx;
use partner_chains_cardano_offchain::csl::{transaction_from_bytes, vkey_witness_from_bytes};
use sidechain_domain::{TransactionCbor, VKeyWitnessCbor};

#[derive(Clone, Debug, clap::Parser)]
pub struct AssembleAndSubmitCmd {
	#[clap(flatten)]
	common_arguments: crate::CommonArguments,
	#[arg(long, value_parser = TransactionCbor::decode_hex)]
	/// Hex-encoded transaction CBOR (with or without 0x prefix)
	transaction: TransactionCbor,
	#[arg(short, long, num_args = 1.., value_delimiter = ' ', value_parser = VKeyWitnessCbor::decode_hex)]
	/// Witnesses of the transaction. Each witness is a hex-encoded CBOR (with or without 0x prefix), encoding a 1 element list containing a 2 elements list [[public_key, signature]].
	witnesses: Vec<VKeyWitnessCbor>,
}

impl AssembleAndSubmitCmd {
	pub async fn execute(self) -> crate::SubCmdResult {
		let client = self.common_arguments.get_ogmios_client().await?;

		let transaction = transaction_from_bytes(self.transaction.0)?;

		let witnesses = self
			.witnesses
			.iter()
			.map(|w| vkey_witness_from_bytes(w.0.clone().into_iter().skip(2).collect()))
			.collect::<Result<Vec<_>, _>>()?;

		let tx_hash =
			assemble_tx(transaction, witnesses, &client, &self.common_arguments.retries()).await?;
		Ok(transaction_submitted_json(tx_hash))
	}
}
