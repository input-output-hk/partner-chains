use crate::key_params::CrossChainSigningKeyParam;
use anyhow::anyhow;
use byte_string::ByteString;
use parity_scale_codec::Encode;
use serde::de::DeserializeOwned;
use serde_json::{self, json};
use sidechain_domain::*;
use sp_block_producer_metadata::MetadataSignedMessage;
use std::io::BufReader;
use time_source::{SystemTimeSource, TimeSource};

/// Generates ECDSA signatures for block producer metadata using cross-chain keys.
#[derive(Clone, Debug, clap::Subcommand)]
#[command(author, version, about, long_about = None)]
pub enum BlockProducerMetadataSignatureCmd<AccountId: FromStrStdErr + Clone + Send + Sync + 'static>
{
	/// Generates signature for the `upsert_metadata` extrinsic
	Upsert {
		/// Genesis UTXO that uniquely identifies the target Partner Chain
		#[arg(long)]
		genesis_utxo: UtxoId,
		/// Path to JSON file containing the metadata to be signed
		#[arg(long)]
		metadata_file: String,
		/// ECDSA private key for cross-chain operations, corresponding to the block producer's identity
		#[arg(long)]
		cross_chain_signing_key: CrossChainSigningKeyParam,
		/// Time-to-live of the signature in seconds.
		#[arg(long, default_value = "3600")]
		ttl: u64,
		/// Partner Chain Account that will be used to upsert the metadata and will own it on-chain
		#[arg(long)]
		partner_chain_account: AccountId,
	},
	/// Generates signature for the `delete_metadata` extrinsic
	Delete {
		/// Genesis UTXO that uniquely identifies the target Partner Chain
		#[arg(long)]
		genesis_utxo: UtxoId,
		/// ECDSA private key for cross-chain operations, corresponding to the block producer's identity
		#[arg(long)]
		cross_chain_signing_key: CrossChainSigningKeyParam,
		/// Time-to-live of the signature in seconds.
		#[arg(long, default_value = "3600")]
		ttl: u64,
		/// Partner Chain Account that will be used to delete the metadata.
		/// It must be the account that owns it on-chain.
		#[arg(long)]
		partner_chain_account: AccountId,
	},
}

impl<AccountId: Encode + FromStrStdErr + Clone + Send + Sync + 'static>
	BlockProducerMetadataSignatureCmd<AccountId>
{
	/// Reads metadata file, generates signatures, and outputs JSON to stdout.
	pub fn execute<M: Send + Sync + DeserializeOwned + Encode>(&self) -> anyhow::Result<()> {
		let input = self.get_input::<M>()?;
		let time_source = SystemTimeSource;
		let output = self.get_output(input, &time_source)?;
		println!("{}", serde_json::to_string_pretty(&output)?);

		Ok(())
	}

	pub fn get_input<M: Send + Sync + DeserializeOwned + Encode>(
		&self,
	) -> anyhow::Result<Option<M>> {
		Ok(match self {
			Self::Upsert { metadata_file, .. } => {
				let file = std::fs::File::open(metadata_file.clone())
					.map_err(|err| anyhow!("Failed to open file {}: {err}", metadata_file))?;
				let metadata_reader = BufReader::new(file);
				let metadata: M = serde_json::from_reader(metadata_reader).map_err(|err| {
					anyhow!("Failed to parse metadata: {err}. Metadata should be in JSON format.",)
				})?;
				Some(metadata)
			},
			Self::Delete { .. } => None,
		})
	}

	/// Generates ECDSA signatures for JSON metadata from reader.
	pub fn get_output<M: Send + Sync + DeserializeOwned + Encode>(
		&self,
		metadata: Option<M>,
		time_source: &impl TimeSource,
	) -> anyhow::Result<serde_json::Value> {
		let encoded_metadata = metadata.as_ref().map(|data| data.encode());
		let message = MetadataSignedMessage {
			cross_chain_pub_key: self.cross_chain_signing_key().vkey(),
			metadata,
			genesis_utxo: *self.genesis_utxo(),
			valid_before: self.valid_before(time_source),
			owner: self.partner_chain_account(),
		};
		let signature = message.sign_with_key(&self.cross_chain_signing_key().0);

		Ok(json!({
			"signature": signature,
			"cross_chain_pub_key": self.cross_chain_signing_key().vkey(),
			"cross_chain_pub_key_hash": self.cross_chain_signing_key().vkey().hash(),
			"encoded_metadata": encoded_metadata.map(ByteString),
			"encoded_message": ByteString(message.encode()),
			"valid_before": self.valid_before(time_source)
		}))
	}

	fn cross_chain_signing_key(&self) -> &CrossChainSigningKeyParam {
		match self {
			Self::Delete { cross_chain_signing_key, .. } => cross_chain_signing_key,
			Self::Upsert { cross_chain_signing_key, .. } => cross_chain_signing_key,
		}
	}

	fn partner_chain_account(&self) -> &AccountId {
		match self {
			Self::Delete { partner_chain_account, .. } => partner_chain_account,
			Self::Upsert { partner_chain_account, .. } => partner_chain_account,
		}
	}

	fn genesis_utxo(&self) -> &UtxoId {
		match self {
			Self::Delete { genesis_utxo, .. } => genesis_utxo,
			Self::Upsert { genesis_utxo, .. } => genesis_utxo,
		}
	}

	fn valid_before(&self, time_source: &impl TimeSource) -> u64 {
		let ttl = match self {
			Self::Delete { ttl, .. } => *ttl,
			Self::Upsert { ttl, .. } => *ttl,
		};
		let now = time_source.get_current_time_millis() / 1000;

		now + ttl
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
	use time_source::MockedTimeSource;

	#[derive(Deserialize, Encode)]
	struct TestMetadata {
		url: String,
		hash: String,
	}

	#[test]
	fn produces_correct_json_output_with_signature_and_pubkey() {
		let time = 100_000_000;
		let ttl = 3600;

		let time_source = MockedTimeSource { current_time_millis: time * 1000 };

		let cmd = BlockProducerMetadataSignatureCmd::Upsert {
			genesis_utxo: UtxoId::new([1; 32], 1),
			metadata_file: "unused".to_string(),
			cross_chain_signing_key: CrossChainSigningKeyParam(
				k256::SecretKey::from_slice(
					// Alice cross-chain key
					&hex!("cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"),
				)
				.unwrap(),
			),
			ttl: 3600,
			partner_chain_account: 999u32,
		};

		let metadata = TestMetadata { url: "http://example.com".into(), hash: "1234".into() };

		let output = cmd.get_output::<TestMetadata>(Some(metadata), &time_source).unwrap();

		let expected_output = json!({
			"cross_chain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1",
			"cross_chain_pub_key_hash" : "0x4a20b7cab322b36838a8e4b6063c3563cdb79c97175f6c2d233dac4d",
			"encoded_message": "0x84020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a10148687474703a2f2f6578616d706c652e636f6d10313233340101010101010101010101010101010101010101010101010101010101010101010010eff50500000000e7030000",
			"signature": "0xcfd171975a2c6ab6757c8ebbf104ba46d8b9722c17b151e4d735fa90673db1183200a6e90578f9337fe5c60d012e06f6b98be902a2d25dcf319f2f1b434bd645",
			"encoded_metadata": "0x48687474703a2f2f6578616d706c652e636f6d1031323334",
			"valid_before": time + ttl
		});

		assert_eq!(output, expected_output)
	}
}
