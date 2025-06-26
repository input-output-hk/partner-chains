//! # Block Producer Metadata Primitives
//!
//! This crate contains primitive types and logic used by the Block Producer Metadata feature
//! of the Partner Chain Toolkit. This feature allows Partner Chain block producers to save
//! information about themselves on-chain. The format of this metadata is left generic for each
//! Partner Chain builder to define.
//!
//! Currently the only code defined in this crate is the [MetadataSignedMessage] type describing
//! the message that is signed and submitted by the block producer together with each change in
//! their on-chain metadata to prove that they are the owner of the public keys associated with
//! the metadata.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use parity_scale_codec::{Decode, Encode};
use sidechain_domain::*;
use sp_api;
extern crate alloc;

/// Message signed to authorize modification of a block producer's on-chain metadata
#[derive(Debug, Clone, Encode)]
pub struct MetadataSignedMessage<Metadata> {
	/// Cross-chain public key
	pub cross_chain_pub_key: CrossChainPublicKey,
	/// Block producer's metadata. [None] signifies a deletion of metadata from the chain.
	pub metadata: Option<Metadata>,
	/// Genesis UTXO of the Partner Chain that the metadata will be submitted to
	pub genesis_utxo: UtxoId,
	/// UNIX epoch time before which a metadata transaction must be executed to the chain to
	/// be valid. This value is mapped to a Partner Chain slot which loses precision for
	/// chains with block times above 1 second.
	pub valid_before: u64,
}

impl<M: Encode> MetadataSignedMessage<M> {
	/// Encodes this message using SCALE codec and signs it
	#[cfg(feature = "std")]
	pub fn sign_with_key(&self, skey: &k256::SecretKey) -> CrossChainSignature {
		use k256::Secp256k1;
		use k256::ecdsa::hazmat::DigestPrimitive;
		use k256::ecdsa::*;
		use k256::sha2::Digest;
		let data = self.encode();
		let digest = <Secp256k1 as DigestPrimitive>::Digest::new_with_prefix(data);

		let (sig, _recid) = SigningKey::from(skey).sign_digest_recoverable(digest.clone()).unwrap();
		CrossChainSignature(sig.to_vec())
	}

	/// Verifies a signature of this message against the given public key
	pub fn verify_signature(
		&self,
		vkey: &CrossChainPublicKey,
		signature: CrossChainSignature,
	) -> Result<(), k256::ecdsa::signature::Error> {
		signature.verify(vkey, &self.encode())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hex_literal::hex;

	#[test]
	fn round_trip() {
		let message = MetadataSignedMessage {
			cross_chain_pub_key: CrossChainPublicKey(vec![1; 32]),
			metadata: Some("metadata".to_string()),
			genesis_utxo: UtxoId::new([2; 32], 0),
			valid_before: 100_000_000_000,
		};

		// Alice cross-chain key
		let skey = k256::SecretKey::from_slice(&hex!(
			"cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"
		))
		.unwrap();
		let vkey = skey.public_key();

		let signature = message.sign_with_key(&skey);

		assert!(message.verify_signature(&vkey.into(), signature).is_ok());
	}
}

sp_api::decl_runtime_apis! {
	/// Runtime API for accessing metadata of block producers
	pub trait BlockProducerMetadataApi<Metadata>
	where Metadata:Decode
	{
		/// Retrieves the metadata for a given SPO public key if it exists.
		fn get_metadata_for(
			cross_chain_pub_key: &CrossChainPublicKey,
		) -> Option<Metadata>;
	}
}
