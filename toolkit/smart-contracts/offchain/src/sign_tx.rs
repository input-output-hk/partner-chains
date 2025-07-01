use crate::{cardano_keys::CardanoPaymentSigningKey, csl::transaction_from_bytes};
use cardano_serialization_lib::{Ed25519Signature, Vkey, Vkeywitness};

/// Signs CBOR encoded Cardano transaction `tx` with `payment_key`.
pub fn sign_tx(tx: Vec<u8>, payment_key: &CardanoPaymentSigningKey) -> anyhow::Result<Vec<u8>> {
	let transaction = transaction_from_bytes(tx)?;
	let tx_hash: [u8; 32] =
		sidechain_domain::crypto::blake2b(transaction.body().to_bytes().as_ref());
	let signature = payment_key.sign(&tx_hash)?;

	let vkey_witness = Vkeywitness::new(
		&Vkey::new(&payment_key.to_csl_pub_key()),
		&Ed25519Signature::from_bytes(signature)?,
	);

	// 0x82 is the tag for a 2-element list in CBOR
	// This is done to keep compatibility with cardano-cli signing commands
	let mut result = vec![0x82, 0x00];
	result.extend(vkey_witness.to_bytes());

	// Return the CBOR binary data
	Ok(result)
}

#[cfg(test)]
mod tests {
	use super::sign_tx;
	use crate::test_values::{
		test_payment_key, test_payment_key2, test_transaction_bytes, test_witness_cbor,
	};
	use hex;

	#[test]
	fn test_sign_tx() {
		// Get test payment key and transaction
		let payment_key = test_payment_key();
		let tx_bytes = test_transaction_bytes();
		let expected_signature_hex = hex::encode(test_witness_cbor()).to_uppercase();

		// Sign the transaction
		let signature_bytes = sign_tx(tx_bytes, &payment_key).unwrap();

		// Check that the signature matches the expected format
		assert_eq!(signature_bytes[0], 0x82); // CBOR 2-element array tag
		assert_eq!(signature_bytes[1], 0x00); // First element is 0 for transaction witnesses

		// Convert the signature to hex for comparison
		let signature_hex = hex::encode(&signature_bytes).to_uppercase();

		// Verify the signature contains the expected signature
		assert_eq!(signature_hex, expected_signature_hex);
	}

	#[test]
	fn test_sign_tx_with_wrong_key() {
		let payment_key = test_payment_key2();
		let tx_bytes = test_transaction_bytes();
		let signature_bytes = sign_tx(tx_bytes, &payment_key).unwrap();
		let unexpected_signature_hex = hex::encode(test_witness_cbor()).to_uppercase();

		// Check that the signature matches the expected format
		assert_eq!(signature_bytes[0], 0x82); // CBOR 2-element array tag
		assert_eq!(signature_bytes[1], 0x00); // First element is 0 for transaction witnesses

		let signature_hex = hex::encode(&signature_bytes).to_uppercase();

		// Verify the signature contains the expected signature
		assert_ne!(signature_hex, unexpected_signature_hex);
	}
}
