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

/// Command structure for generating block producer metadata signatures.
///
/// This struct represents the parameters required to cryptographically sign
/// block producer metadata for submission to the Partner Chain runtime.
/// The metadata signature process ensures that metadata updates can only
/// be performed by authorized block producers who possess the corresponding
/// private keys.
///
/// ## Metadata Signing Process
///
/// The command performs the following operations:
/// 1. Reads metadata from a JSON file
/// 2. SCALE-encodes the metadata for deterministic serialization
/// 3. Creates a signed message structure including the cross-chain public key
/// 4. Generates an ECDSA signature using the block producer's private key
/// 5. Outputs structured JSON with signatures and encoded data
///
/// ## Cross-Chain Integration
///
/// The signature is generated using an ECDSA key that corresponds to the
/// block producer's cross-chain identity, enabling verification across
/// both Cardano and Partner Chain networks.
#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct BlockProducerMetadataSignatureCmd {
	/// Genesis UTXO that uniquely identifies the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Path to JSON file containing the metadata to be signed
	#[arg(long)]
	pub metadata_file: String,
	/// ECDSA private key for cross-chain operations, corresponding to the block producer's identity
	#[arg(long)]
	pub cross_chain_signing_key: CrossChainSigningKeyParam,
}

impl BlockProducerMetadataSignatureCmd {
	/// Executes the metadata signature generation process.
	///
	/// This method performs the complete workflow for signing block producer metadata:
	/// 1. Opens and reads the specified metadata file
	/// 2. Processes the metadata through the signature generation pipeline
	/// 3. Outputs the resulting signatures and encoded data as JSON
	///
	/// # Type Parameters
	/// * `M` - The metadata type that implements the required traits for deserialization and encoding
	///
	/// # Returns
	/// * `Ok(())` - Successful execution with JSON output printed to stdout
	/// * `Err(anyhow::Error)` - File I/O error, JSON parsing error, or signature generation failure
	///
	/// # Errors
	/// This method will return an error if:
	/// - The metadata file cannot be opened or read
	/// - The file content is not valid JSON matching the expected metadata type
	/// - JSON serialization of the output fails
	pub fn execute<M: Send + Sync + DeserializeOwned + Encode>(&self) -> anyhow::Result<()> {
		let file = std::fs::File::open(self.metadata_file.clone())
			.map_err(|err| anyhow!("Failed to open file {}: {err}", self.metadata_file))?;
		let metadata_reader = BufReader::new(file);
		let output = self.get_output::<M>(metadata_reader)?;

		println!("{}", serde_json::to_string_pretty(&output)?);

		Ok(())
	}

	/// Processes metadata from a reader and generates cryptographic signatures.
	///
	/// This method handles the core logic of metadata signature generation:
	/// 1. Deserializes JSON metadata from the provided reader
	/// 2. SCALE-encodes the metadata for deterministic serialization
	/// 3. Creates a signed message structure with cross-chain public key and genesis UTXO
	/// 4. Generates ECDSA signature using the block producer's private key
	/// 5. Constructs structured output with all relevant cryptographic material
	///
	/// # Type Parameters
	/// * `M` - Metadata type that must be deserializable from JSON and SCALE-encodable
	///
	/// # Arguments
	/// * `metadata_reader` - Reader providing JSON-formatted metadata content
	///
	/// # Returns
	/// * `Ok(serde_json::Value)` - JSON object containing signatures and encoded data
	/// * `Err(anyhow::Error)` - Deserialization or signature generation error
	///
	/// # Output Structure
	/// The returned JSON contains:
	/// - `signature`: ECDSA signature over the complete message
	/// - `cross_chain_pub_key`: Public key corresponding to the signing key
	/// - `cross_chain_pub_key_hash`: Hash of the public key for efficient lookups
	/// - `encoded_metadata`: SCALE-encoded metadata bytes
	/// - `encoded_message`: SCALE-encoded complete signed message
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
