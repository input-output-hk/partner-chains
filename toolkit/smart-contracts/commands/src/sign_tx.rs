use crate::PaymentFilePath;
use partner_chains_cardano_offchain::sign_tx::sign_tx;
use serde_json::json;
use sidechain_domain::TransactionCbor;

#[derive(Clone, Debug, clap::Parser)]
/// Command for signing a cardano transaction
pub struct SignTxCmd {
	#[arg(long)]
	/// Hex-encoded transaction CBOR (with or without 0x prefix), or path to a JSON file containing transaction_to_sign output
	transaction: String,
	#[clap(flatten)]
	/// Path to the Cardano Signing Key file that you want to sign the transaction with
	payment_key_file: PaymentFilePath,
}

impl SignTxCmd {
	/// Signs a cardano transaction
	pub async fn execute(self) -> crate::SubCmdResult {
		let payment_key = self.payment_key_file.read_key()?;

		// Try to extract cborHex from the input
		let cbor_hex = extract_cbor_hex(&self.transaction)?;
		let transaction_cbor: TransactionCbor = cbor_hex.parse()
			.map_err(|e| anyhow::anyhow!("Failed to parse transaction CBOR: {}", e))?;

		let vkey_witness = sign_tx(transaction_cbor.0, &payment_key)?;

		let json = json!(
			{
				"type": "TxWitness ConwayEra",
				"description": "",
				"cborHex": hex::encode(vkey_witness)
			}
		);
		Ok(json)
	}
}

/// Extracts cborHex from the input string.
/// Handles three cases:
/// 1. Direct hex string (with or without 0x prefix)
/// 2. JSON string containing transaction_to_sign structure
/// 3. Path to a file containing JSON
fn extract_cbor_hex(input: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
	let trimmed = input.trim();
	
	// Case 1: Check if it's a direct hex string
	if !trimmed.starts_with('{') && !std::path::Path::new(trimmed).exists() {
		return Ok(trimmed.to_string());
	}
	
	// Case 2 & 3: Try to parse as JSON (either directly or from file)
	let json_str = if std::path::Path::new(trimmed).exists() {
		std::fs::read_to_string(trimmed)?
	} else {
		trimmed.to_string()
	};
	
	let json_value: serde_json::Value = serde_json::from_str(&json_str)?;
	
	// Try to extract cborHex from transaction_to_sign format
	if let Some(cbor_hex) = json_value
		.get("transaction_to_sign")
		.and_then(|v| v.get("tx"))
		.and_then(|v| v.get("cborHex"))
		.and_then(|v| v.as_str())
	{
		return Ok(cbor_hex.to_string());
	}
	
	// Try to extract cborHex from direct tx format
	if let Some(cbor_hex) = json_value.get("cborHex").and_then(|v| v.as_str()) {
		return Ok(cbor_hex.to_string());
	}
	
	Err("Could not extract cborHex from input. Expected hex string, transaction_to_sign JSON, or file path.".into())
}
