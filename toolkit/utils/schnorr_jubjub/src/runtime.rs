//! Implements runtime traits required by the Substrate framework.
//!
//! This module provides trait implementations that integrate cryptographic
//! primitives and keystore functionality into the Substrate runtime
//! environment, enabling their use in on-chain logic and consensus mechanisms.

use std::convert::TryInto;

use rand_core::{OsRng, RngCore};
use sp_core::{
	ByteArray, Pair as TraitPair,
	crypto::{CryptoTypeId, KeyTypeId},
};
use sp_externalities::ExternalitiesExt;
use sp_keystore::KeystoreExt;
use sp_runtime::app_crypto::RuntimePublic;
use sp_runtime_interface::{
	pass_by::{AllocateAndReturnByCodec, PassFatPointerAndRead, PassPointerAndReadCopy},
	runtime_interface,
};

use crate::poseidon::PoseidonJubjub;
use crate::{
	beefy_structures::{CRYPTO_ID, InnerPublicBytes, Public, Signature},
	primitive::{SchnorrSignature, VerifyingKey},
};

#[runtime_interface]
pub trait GenericKeyInterface {
	fn keys(
		&mut self,
		id: PassPointerAndReadCopy<KeyTypeId, 4>,
	) -> AllocateAndReturnByCodec<Vec<Vec<u8>>> {
		self.extension::<KeystoreExt>()
			.expect("No `keystore` associated for the current context!")
			.keys(id)
			.expect("Key type not found in keystore")
	}

	fn insert(
		&mut self,
		id: PassPointerAndReadCopy<KeyTypeId, 4>,
		suri: PassPointerAndReadCopy<[u8; 64], 64>,
		public: PassPointerAndReadCopy<InnerPublicBytes, 32>,
	) {
		self.extension::<KeystoreExt>()
			.expect("No `keystore` associated for the current context!")
			.insert(id, &hex::encode(&suri), public.as_ref())
			.expect("Failed to insert key in keystore")
	}

	fn sign_with(
		&mut self,
		id: PassPointerAndReadCopy<KeyTypeId, 4>,
		crypto_id: PassPointerAndReadCopy<[u8; 4], 4>,
		public: PassPointerAndReadCopy<InnerPublicBytes, 32>,
		msg: PassFatPointerAndRead<&[u8]>,
	) -> AllocateAndReturnByCodec<Option<Vec<u8>>> {
		self.extension::<KeystoreExt>()
			.expect("No `keystore` associated for the current context!")
			.sign_with(id, CryptoTypeId(crypto_id), public.as_ref(), msg)
			.expect("Failed to produce valid signature")
	}
}

impl RuntimePublic for Public {
	type Signature = Signature;

	fn all(key_type: KeyTypeId) -> Vec<Self> {
		let all = generic_key_interface::keys(key_type);

		all.iter()
			.map(|bytes| Public::try_from(bytes.as_slice()).expect("Invalid format in keystore"))
			.collect::<Vec<_>>()
	}

	fn generate_pair(key_type: KeyTypeId, seed: Option<Vec<u8>>) -> Self {
		let seed: [u8; 64] = seed
			.unwrap_or({
				let mut res = [0u8; 64];
				OsRng.fill_bytes(&mut res);

				res.to_vec()
			})
			.try_into()
			.expect("Invalid seed size.");

		let keypair = crate::primitive::KeyPair::generate_from_seed(seed);
		generic_key_interface::insert(key_type, seed, keypair.public().0);

		keypair.public()
	}

	fn sign<M: AsRef<[u8]>>(&self, key_type: KeyTypeId, msg: &M) -> Option<Self::Signature> {
		let crypto_id = CRYPTO_ID;
		let bytes = generic_key_interface::sign_with(key_type, crypto_id.0, self.0, msg.as_ref())?;

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
