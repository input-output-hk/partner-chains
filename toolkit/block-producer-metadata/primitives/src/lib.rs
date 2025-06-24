#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use sidechain_domain::*;
use sp_api;
extern crate alloc;

#[derive(Debug, Clone, Encode)]
pub struct MetadataSignedMessage<Metadata> {
	pub cross_chain_pub_key: CrossChainPublicKey,
	pub metadata: Option<Metadata>,
	pub genesis_utxo: UtxoId,
}

impl<M: Encode> MetadataSignedMessage<M> {
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
			metadata: "metadata".to_string(),
			genesis_utxo: UtxoId::new([2; 32], 0),
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
	pub trait BlockProducerMetadataApi<Metadata>
	where Metadata:Decode
	{
		/// Retrieves the metadata for a given SPO public key if it exists.
		fn get_metadata_for(
			cross_chain_pub_key: &CrossChainPublicKey,
		) -> Option<Metadata>;
	}
}
