use parity_scale_codec::Encode;
use secp256k1::{hashes::sha256, Message, Secp256k1};
use sidechain_domain::*;

#[derive(Debug, Clone, Encode)]
pub struct MetadataSignedMessage<Metadata> {
	pub cross_chain_pub_key: CrossChainPublicKey,
	pub metadata: Metadata,
	pub genesis_utxo: UtxoId,
}

impl<M: Encode> MetadataSignedMessage<M> {
	pub fn sign_with_key(&self, skey: &secp256k1::SecretKey) -> CrossChainSignature {
		let data = self.encode();
		let data_hash = Message::from_hashed_data::<sha256::Hash>(&data);
		CrossChainSignature(skey.sign_ecdsa(data_hash).serialize_der().into_iter().collect())
	}

	pub fn verify_signature(
		&self,
		vkey: &secp256k1::PublicKey,
		signature: CrossChainSignature,
	) -> bool {
		let data = self.encode();
		let data_hash = Message::from_hashed_data::<sha256::Hash>(&data);

		println!("{}", signature.0.len());
		let signature = secp256k1::ecdsa::Signature::from_der(&signature.0)
			.or_else(|_| secp256k1::ecdsa::Signature::from_compact(&signature.0))
			.expect("ecdsa::Signature from CrossChainSignature should always succeed");

		vkey.verify(&Secp256k1::new(), &data_hash, &signature).is_ok()
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
		let skey = secp256k1::SecretKey::from_slice(&hex!(
			"cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854"
		))
		.unwrap();
		let vkey = skey.public_key(&Secp256k1::new());

		let signature = message.sign_with_key(&skey);

		assert!(message.verify_signature(&vkey, signature));
	}
}
