#![allow(unused_imports)]
use crate::key_params::{
	CrossChainSigningKeyParam, SidechainSigningKeyParam, StakePoolSigningKeyParam,
};
use byte_string::ByteString;
use clap::Parser;
use parity_scale_codec::Encode;
use plutus_datum_derive::*;
use secp256k1::{hashes::sha256, Message, SecretKey};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{self, json};
use sidechain_domain::{crypto::*, *};
use sp_block_producer_metadata::MetadataSignedMessage;
use std::{
	fmt::{Display, Formatter},
	io::{BufReader, Read},
	marker::PhantomData,
};

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct BlockProducerMetadataSignatureCmd {
	/// Genesis UTXO of the target Partner Chain
	#[arg(long)]
	pub genesis_utxo: UtxoId,
	/// Path of the file containing the metadata in JSON format
	#[arg(long)]
	pub metadata_file: String,
	/// ECDSA signing key of the
	#[arg(long)]
	pub cross_chain_signing_key: CrossChainSigningKeyParam,
}

impl BlockProducerMetadataSignatureCmd {
	pub fn execute<M: Send + Sync + DeserializeOwned + Encode>(&self) -> anyhow::Result<()> {
		let metadata_reader = BufReader::new(std::fs::File::open(self.metadata_file.clone())?);
		let output = self.get_output::<M>(metadata_reader)?;

		println!("{}", serde_json::to_string_pretty(&output)?);

		Ok(())
	}

	pub fn get_output<M: Send + Sync + DeserializeOwned + Encode>(
		&self,
		metadata_reader: impl Read,
	) -> anyhow::Result<serde_json::Value> {
		let metadata: M = serde_json::from_reader(metadata_reader)?;
		let encoded_metadata = metadata.encode();
		let message = MetadataSignedMessage {
			cross_chain_pub_key: self.cross_chain_signing_key.vkey(),
			metadata,
			genesis_utxo: self.genesis_utxo.clone(),
		};
		let signature = message.sign_with_key(&self.cross_chain_signing_key.0);

		Ok(json!({
			"signature": signature,
			"cross_chain_pub_key": self.cross_chain_signing_key.vkey(),
			"encoded_metadata": ByteString(encoded_metadata)
		}))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::key_params::CrossChainSigningKeyParam;
	use hex_literal::hex;
	use pretty_assertions::assert_eq;
	use serde::{Deserialize, Serialize};
	use serde_json::json;
	use sidechain_domain::UtxoId;
	use std::{
		io::{BufReader, BufWriter},
		str::FromStr,
	};

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
				secp256k1::SecretKey::from_slice(
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
			"cross_chain_pub_key": "0x0a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a12da8d65fae6d63a4abca410b7e50d50cd95d36001c28712fd2adf944adb03b12",
			"signature": "0x304502210085bbfc2df4e11bf6f6c5fa496b37fab97392dd59601a30dcf9c45aa40fc0fa65022079d7f692bc72e04a16dc3f1676cc17c5709859aa03204115432f6f88362e4831",
			"encoded_metadata": "0x48687474703a2f2f6578616d706c652e636f6d1031323334"
		});

		assert_eq!(output, expected_output)
	}
}
