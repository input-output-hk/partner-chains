use crate::PaymentFilePath;
use partner_chains_cardano_offchain::sign_tx::sign_tx;
use serde_json::json;
use sidechain_domain::TransactionCbor;

#[derive(Clone, Debug, clap::Parser)]
/// Command for signing a cardano transaction
pub struct SignTxCmd {
	#[arg(
		long,
		value_hint = clap::ValueHint::AnyPath,
		help = "Transaction input: hex string, JSON with transaction_to_sign, or file path to JSON"
	)]
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
/// Uses deterministic order: file path -> JSON parsing -> hex string
fn extract_cbor_hex(input: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
	let trimmed = input.trim();
	
	// Case 1: Check if it's a file path that exists
	let json_str = if std::path::Path::new(trimmed).exists() {
		std::fs::read_to_string(trimmed)
			.map_err(|e| format!("Failed to read file '{}': {}", trimmed, e))?
	} else if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(trimmed) {
		// Case 2: Try parsing as JSON directly
		return extract_cbor_from_json(&json_value);
	} else {
		// Case 3: Treat as direct hex string
		return Ok(trimmed.to_string());
	};
	
	// If we read from file, parse the JSON and extract cborHex
	let json_value: serde_json::Value = serde_json::from_str(&json_str)
		.map_err(|e| format!("Failed to parse JSON from file: {}", e))?;
	
	extract_cbor_from_json(&json_value)
}

/// Extracts cborHex from a JSON value
fn extract_cbor_from_json(json_value: &serde_json::Value) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
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
	
	Err("Could not extract cborHex from JSON. Expected 'transaction_to_sign.tx.cborHex' or 'cborHex' field.".into())
}
