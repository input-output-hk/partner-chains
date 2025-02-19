use plutus::to_datum_cbor_bytes;
use plutus::ToDatum;
use secp256k1::{ecdsa::Signature, Message, PublicKey, SecretKey};
use sidechain_domain::{MainchainSignature, StakePoolPublicKey};
use sp_io::hashing::blake2_256;

pub fn hash<T: ToDatum>(msg: T) -> [u8; 32] {
	blake2_256(to_datum_cbor_bytes(msg).as_slice())
}

pub fn sc_public_key_and_signature(key: SecretKey, hashed: [u8; 32]) -> (PublicKey, Signature) {
	let public_key = PublicKey::from_secret_key_global(&key);
	let signature = key.sign_ecdsa(Message::from_digest_slice(hashed.as_slice()).unwrap());
	(public_key, signature)
}

pub fn sc_public_key_and_signature_for_datum<T: ToDatum>(
	key: SecretKey,
	datum_msg: T,
) -> (PublicKey, Signature) {
	let hashed_msg = hash(datum_msg);
	sc_public_key_and_signature(key, hashed_msg)
}

pub fn cardano_spo_public_key_and_signature<T: ToDatum>(
	key: ed25519_zebra::SigningKey,
	datum_msg: T,
) -> (StakePoolPublicKey, MainchainSignature) {
	let message = to_datum_cbor_bytes(datum_msg);
	let signature = MainchainSignature(key.sign(&message).into());
	let public = StakePoolPublicKey(ed25519_zebra::VerificationKey::from(&key).into());
	(public, signature)
}
