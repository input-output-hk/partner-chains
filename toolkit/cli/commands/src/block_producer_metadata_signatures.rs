//! # Block Producer Metadata Signatures
//!
//! Generate ECDSA signatures for block producer metadata registration.
//! This module enables block producers to sign their metadata with cross-chain keys
//! for secure metadata updates and verification.
//!
//! ## Process Overview
//!
//! 1. Load metadata from JSON file
//! 2. Create signed message with cross-chain public key, metadata, and genesis UTXO
//! 3. Sign message using ECDSA cross-chain signing key
//! 4. Output signature, public key hash, and encoded data in JSON format
//!
//! ## Metadata File Format
//!
//! ```json
//! {
//!   "url": "http://example.com/metadata",
//!   "hash": "1234567890abcdef"
//! }
//! ```
//!
//! ## CLI Integration
//!
//! ```bash
//! partner-chains-cli block-producer-metadata-signature \
//!   --genesis-utxo 0101010101010101010101010101010101010101010101010101010101010101#0 \
//!   --metadata-file metadata.json \
//!   --cross-chain-signing-key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
//! ```
//!
//! ## Output Format
//!
//! ```json
//! {
//!   "signature": "0xf86d3aa75a6a8bda35dfdd2472b8e5f2f95446e4542ab0adb6f3e7681f01b74060082c0debfb9616a54f88cf42b88e1a2f43c75dc4394bfdde33972deb491fcb",
//!   "cross_chain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
//!   "cross_chain_pub_key_hash": "0x4a20b7cab322b36838a8e4b6063c3563cdb79c97175f6c2d233dac4d",
//!   "encoded_metadata": "0x48687474703a2f2f6578616d706c652e636f6d1031323334",
//!   "encoded_message": "0x84020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a148687474703a2f2f6578616d706c652e636f6d103132333401010101010101010101010101010101010101010101010101010101010101010100"
//! }
//! ```

use crate::key_params::CrossChainSigningKeyParam;
use anyhow::anyhow;
use byte_string::ByteString;
use clap::Parser;
use parity_scale_codec::Encode;
use serde::de::DeserializeOwned;
use serde_json::{self, json};
use sidechain_domain::*;
use sp_block_producer_metadata::MetadataSignedMessage;
use std::io::{BufReader, Read};

/// Command for generating block producer metadata signatures.
///
/// Signs block producer metadata using ECDSA cross-chain keys to authorize metadata updates.
/// The metadata file must contain valid JSON that can be deserialized to the target metadata type.
///
/// ## Parameters
///
/// - `genesis_utxo`: Identifies the target Partner Chain
/// - `metadata_file`: Path to JSON file containing block producer metadata
/// - `cross_chain_signing_key`: ECDSA signing key for cross-chain operations
///
/// ## Example Usage
///
/// ```bash
/// partner-chains-cli block-producer-metadata-signature \
///   --genesis-utxo 0101010101010101010101010101010101010101010101010101010101010101#0 \
///   --metadata-file metadata.json \
///   --cross-chain-signing-key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
/// ```
///
/// ## Metadata File Requirements
///
/// The metadata file must contain valid JSON matching the expected metadata structure.
/// Common metadata fields include `url` and `hash` for referencing external metadata.
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct BlockProducerMetadataSignatureCmd {
	/// Genesis UTXO of the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Path of the file containing the metadata in JSON format
	#[arg(long)]
	pub metadata_file: String,
	/// ECDSA signing key of the block producer, corresponding to the public key that will be associated with new metadata
	#[arg(long)]
	pub cross_chain_signing_key: CrossChainSigningKeyParam,
}

impl BlockProducerMetadataSignatureCmd {
	/// Execute the block producer metadata signature command.
	///
	/// Reads metadata from the specified file, generates a cryptographic signature,
	/// and outputs the result as formatted JSON. The metadata must be valid JSON
	/// that deserializes to the target metadata type.
	///
	/// ## Process
	///
	/// 1. Open and read metadata file
	/// 2. Parse JSON metadata content
	/// 3. Generate ECDSA signature with cross-chain key
	/// 4. Output JSON with signature and metadata information
	///
	/// ## Type Parameters
	///
	/// - `M`: Metadata type that implements `Send + Sync + DeserializeOwned + Encode`
	///
	/// ## Errors
	///
	/// Returns `anyhow::Error` if:
	/// - File cannot be opened or read
	/// - JSON parsing fails
	/// - JSON serialization of output fails
	///
	/// ## Example Output
	///
	/// ```json
	/// {
	///   "signature": "0xf86d3aa75a6a8bda35dfdd2472b8e5f2f95446e4542ab0adb6f3e7681f01b74060082c0debfb9616a54f88cf42b88e1a2f43c75dc4394bfdde33972deb491fcb",
	///   "cross_chain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
	///   "cross_chain_pub_key_hash": "0x4a20b7cab322b36838a8e4b6063c3563cdb79c97175f6c2d233dac4d",
	///   "encoded_metadata": "0x48687474703a2f2f6578616d706c652e636f6d1031323334",
	///   "encoded_message": "0x84020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a148687474703a2f2f6578616d706c652e636f6d103132333401010101010101010101010101010101010101010101010101010101010101010100"
	/// }
	/// ```
	pub fn execute<M: Send + Sync + DeserializeOwned + Encode>(&self) -> anyhow::Result<()> {
		let file = std::fs::File::open(self.metadata_file.clone())
			.map_err(|err| anyhow!("Failed to open file {}: {err}", self.metadata_file))?;
		let metadata_reader = BufReader::new(file);
		let output = self.get_output::<M>(metadata_reader)?;

		println!("{}", serde_json::to_string_pretty(&output)?);

		Ok(())
	}

	/// Generate signature and output data for block producer metadata.
	///
	/// Parses metadata from the provided reader, creates a signed message with the cross-chain
	/// public key and genesis UTXO, then generates an ECDSA signature. Returns all relevant
	/// data including the signature, public key information, and encoded message data.
	///
	/// ## Process
	///
	/// 1. Deserialize metadata from JSON reader
	/// 2. SCALE-encode metadata for signature generation
	/// 3. Create `MetadataSignedMessage` with cross-chain public key, metadata, and genesis UTXO
	/// 4. Generate ECDSA signature using cross-chain signing key
	/// 5. Return JSON object with signature and metadata information
	///
	/// ## Type Parameters
	///
	/// - `M`: Metadata type that implements `Send + Sync + DeserializeOwned + Encode`
	///
	/// ## Parameters
	///
	/// - `metadata_reader`: Reader containing JSON metadata content
	///
	/// ## Returns
	///
	/// `serde_json::Value` containing:
	/// - `signature`: ECDSA signature bytes in hex format
	/// - `cross_chain_pub_key`: Cross-chain public key in hex format
	/// - `cross_chain_pub_key_hash`: Hash of the cross-chain public key
	/// - `encoded_metadata`: SCALE-encoded metadata bytes
	/// - `encoded_message`: SCALE-encoded signed message bytes
	///
	/// ## Errors
	///
	/// Returns `anyhow::Error` if:
	/// - JSON deserialization fails
	/// - Metadata structure is invalid
	pub fn get_output<M: Send + Sync + DeserializeOwned + Encode>(
		&self,
		metadata_reader: impl Read,
	) -> anyhow::Result<serde_json::Value> {
		let metadata: M = serde_json::from_reader(metadata_reader).map_err(|err| {
			anyhow!("Failed to parse metadata: {err}. Metadata should be in JSON format.",)
		})?;
		let encoded_metadata = metadata.encode();
		let message = MetadataSignedMessage {
			cross_chain_pub_key: self.cross_chain_signing_key.vkey(),
			metadata,
			genesis_utxo: self.genesis_utxo,
		};
		let signature = message.sign_with_key(&self.cross_chain_signing_key.0);

		Ok(json!({
			"signature": signature,
			"cross_chain_pub_key": self.cross_chain_signing_key.vkey(),
			"cross_chain_pub_key_hash": self.cross_chain_signing_key.vkey().hash(),
			"encoded_metadata": ByteString(encoded_metadata),
			"encoded_message": ByteString(message.encode())
		}))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::key_params::CrossChainSigningKeyParam;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;
	use serde::Deserialize;
	use serde_json::json;
	use sidechain_domain::UtxoId;

	#[derive(Deserialize, Encode)]
	struct TestMetadata {
		url: String,
		hash: String,
	}

	#[test]
	fn produces_correct_json_output_with_signature_and_pubkey() {
		let cmd = BlockProducerMetadataSignatureCmd {
			genesis_utxo: UtxoId::new([1; 32], 1),
			metadata_file: "unused".to_string(),
			cross_chain_signing_key: CrossChainSigningKeyParam(
				k256::SecretKey::from_slice(
					// Alice cross-chain key
					&hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"),
				)
				.unwrap(),
			),
		};

		let metadata_json = json!({
			"url": "http://example.com",
			"hash": "1234"
		});
		let metadata = serde_json::to_string(&metadata_json).unwrap();
		let metadata_reader = BufReader::new(metadata.as_bytes());

		let output = cmd.get_output::<TestMetadata>(metadata_reader).unwrap();

		let expected_output = json!({
			"cross_chain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
			"cross_chain_pub_key_hash" : "0x4a20b7cab322b36838a8e4b6063c3563cdb79c97175f6c2d233dac4d",
			"encoded_message": "0x84020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a148687474703a2f2f6578616d706c652e636f6d103132333401010101010101010101010101010101010101010101010101010101010101010100",
			"signature": "0xf86d3aa75a6a8bda35dfdd2472b8e5f2f95446e4542ab0adb6f3e7681f01b74060082c0debfb9616a54f88cf42b88e1a2f43c75dc4394bfdde33972deb491fcb",
			"encoded_metadata": "0x48687474703a2f2f6578616d706c652e636f6d1031323334"
		});

		assert_eq!(output, expected_output)
	}

	#[test]
	fn retuns_error_for_invalid_json() {
		let cmd = BlockProducerMetadataSignatureCmd {
			genesis_utxo: UtxoId::new([1; 32], 1),
			metadata_file: "unused".to_string(),
			cross_chain_signing_key: CrossChainSigningKeyParam(
				k256::SecretKey::from_slice(
					// Alice cross-chain key
					&hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"),
				)
				.unwrap(),
			),
		};

		let metadata_reader = BufReader::new("{ invalid json }".as_bytes());

		let output = cmd.get_output::<TestMetadata>(metadata_reader);

		assert!(output.is_err(), "{:?} should be Err", output);
	}
}
