//! Implements runtime traits required by the Substrate framework.
//!
//! This module provides trait implementations that integrate cryptographic
//! primitives and keystore functionality into the Substrate runtime
//! environment, enabling their use in on-chain logic and consensus mechanisms.

// use std::convert::TryInto;

use alloc::vec::Vec;
use sp_core::{
	ByteArray, Pair as TraitPair,
	crypto::KeyTypeId,
};
use sp_runtime::app_crypto::RuntimePublic;

use crate::poseidon::PoseidonJubjub;
use crate::{
	beefy_structures::{CRYPTO_ID, Public, Signature},
	primitive::{SchnorrSignature, VerifyingKey},
};

impl RuntimePublic for Public {
	type Signature = Signature;

	fn all(key_type: KeyTypeId) -> Vec<Self> {
		let all = sp_io::generic_crypto::keys(key_type);

		all.iter()
			.map(|bytes| Public::try_from(bytes.as_slice()).expect("Invalid format in keystore"))
			.collect::<Vec<_>>()
	}

	fn generate_pair(key_type: KeyTypeId, seed: Option<Vec<u8>>) -> Self {
		let unwrapped_seed = core::str::from_utf8(&seed.clone().expect("Only support key generation from given seed.")).expect("Seed contains non-UTF8 characters");

		let keypair = crate::primitive::KeyPair::generate_from_seed(unwrapped_seed.as_bytes());
		sp_io::generic_crypto::insert(key_type, seed, &keypair.public().0);

		keypair.public()
	}

	fn sign<M: AsRef<[u8]>>(&self, key_type: KeyTypeId, msg: &M) -> Option<Self::Signature> {
		let crypto_id = CRYPTO_ID;
		let bytes = sp_io::generic_crypto::sign_with(key_type, crypto_id.0, &self.0, msg.as_ref())?;

		Signature::try_from(bytes.as_ref()).ok()
	}

	fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool {
		let msg = PoseidonJubjub::msg_from_bytes(msg.as_ref(), false)
			.expect("With flag set to false, this should not fail. Report a bug.");

		let sig = SchnorrSignature::from_bytes(signature.as_ref());
		let pubkey = VerifyingKey::from_bytes(self.as_ref());

		match (sig, pubkey) {
			(Ok(sig), Ok(pubkey)) => sig.verify(&msg, &pubkey).is_ok(),
			(_, _) => false,
		}
	}

	fn generate_proof_of_possession(&mut self, _key_type: KeyTypeId) -> Option<Self::Signature> {
		unimplemented!()
	}

	fn verify_proof_of_possession(&self, _pop: &Self::Signature) -> bool {
		unimplemented!()
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		self.as_slice().to_vec()
	}
}

// #[runtime_interface]
// pub trait GenericKeyInterface {
// 	fn keys(
// 		&mut self,
// 		id: PassPointerAndReadCopy<KeyTypeId, 4>,
// 	) -> AllocateAndReturnByCodec<Vec<Vec<u8>>> {
// 		self.extension::<KeystoreExt>()
// 			.expect("No `keystore` associated for the current context!")
// 			.keys(id)
// 			.expect("Key type not found in keystore")
// 	}
//
// 	fn insert(
// 		&mut self,
// 		id: PassPointerAndReadCopy<KeyTypeId, 4>,
// 		suri: PassFatPointerAndDecode<Option<Vec<u8>>>,
// 		public: PassPointerAndReadCopy<InnerPublicBytes, 32>,
// 	) {
// 		let seed = suri.expect("Jubjub only working with provided seed");
// 		let seed = core::str::from_utf8(&seed).expect("Seed is valid utf8!");
// 		self.extension::<KeystoreExt>()
// 			.expect("No `keystore` associated for the current context!")
// 			.insert(id, seed, public.as_ref())
// 			.expect("Failed to insert key in keystore")
// 	}
//
// 	fn sign_with(
// 		&mut self,
// 		id: PassPointerAndReadCopy<KeyTypeId, 4>,
// 		crypto_id: PassPointerAndReadCopy<[u8; 4], 4>,
// 		public: PassPointerAndReadCopy<InnerPublicBytes, 32>,
// 		msg: PassFatPointerAndRead<&[u8]>,
// 	) -> AllocateAndReturnByCodec<Option<Vec<u8>>> {
// 		self.extension::<KeystoreExt>()
// 			.expect("No `keystore` associated for the current context!")
// 			.sign_with(id, CryptoTypeId(crypto_id), public.as_ref(), msg)
// 			.expect("Failed to produce valid signature")
// 	}
// }

