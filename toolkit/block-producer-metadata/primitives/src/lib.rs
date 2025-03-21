#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::Encode;
use secp256k1::{hashes::sha256, Message};
use sidechain_domain::*;

#[derive(Debug, Clone, Encode)]
pub struct MetadataSignedMessage<Metadata> {
	pub cross_chain_pub_key: CrossChainPublicKey,
	pub metadata: Metadata,
	pub genesis_utxo: UtxoId,
}

impl<M: Encode> MetadataSignedMessage<M> {
	#[cfg(feature = "std")]
	pub fn sign_with_key(&self, skey: &secp256k1::SecretKey) -> CrossChainSignature {
		let data = self.encode();
		let data_hash = Message::from_hashed_data::<sha256::Hash>(&data);
		CrossChainSignature(skey.sign_ecdsa(data_hash).serialize_der().into_iter().collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use hex_literal::hex;
	use secp256k1::Secp256k1;

	#[test]
	fn round_trip() {
		let message = MetadataSignedMessage {
			cross_chain_pub_key: CrossChainPublicKey(vec![1; 32]),
			metadata: "metadata".to_string(),
			genesis_utxo: UtxoId::new([2; 32], 0),
		};

		// Alice cross-chain key
		let skey = secp256k1::SecretKey::from_slice(&hex!(
			"cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"
		))
		.unwrap();
		let vkey = skey.public_key(&Secp256k1::new());

		let signature = message.sign_with_key(&skey);

		assert!(signature.verify(&vkey.into(), &message.encode()));
	}
}
