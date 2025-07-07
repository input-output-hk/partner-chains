//! Wrapper around the `LocalKeystore`.
//!
//! This keystore delegates to the underlying `LocalKeystore` for most
//! functionality, but re-implements the `sign_with` method to support Schnorr
//! signatures over the JubJub curve, which the default implementation does not
//! support due to lack of genericity.

use alloc::vec::Vec;
use std::sync::Arc;
use sc_keystore::LocalKeystore;
use sp_application_crypto::Pair as TraitPair;
use sp_core::{
	crypto::{ByteArray, CryptoTypeId, KeyTypeId},
	ecdsa, ed25519, sr25519,
};
use sp_keystore::{Error, Keystore};

use crate::{
	beefy_structures::{CRYPTO_ID, InnerPublicBytes, Public},
	primitive::KeyPair as Pair,
};

/// Wrapper of the LocalKeystore
// TRY TO CHANGE THIS TO KeyStorePtr
pub struct SchnorrKeystore(pub Arc<LocalKeystore>);

impl Keystore for SchnorrKeystore {
	fn sr25519_public_keys(&self, key_type: KeyTypeId) -> Vec<sr25519::Public> {
		self.0.sr25519_public_keys(key_type)
	}

	fn sr25519_generate_new(
		&self,
		key_type: KeyTypeId,
		seed: Option<&str>,
	) -> Result<sr25519::Public, Error> {
		self.0.sr25519_generate_new(key_type, seed)
	}

	fn sr25519_sign(
		&self,
		key_type: KeyTypeId,
		public: &sr25519::Public,
		msg: &[u8],
	) -> Result<Option<sr25519::Signature>, Error> {
		self.0.sr25519_sign(key_type, public, msg)
	}

	fn sr25519_vrf_sign(
		&self,
		key_type: KeyTypeId,
		public: &sr25519::Public,
		data: &sr25519::vrf::VrfSignData,
	) -> Result<Option<sr25519::vrf::VrfSignature>, Error> {
		self.0.sr25519_vrf_sign(key_type, public, data)
	}

	fn sr25519_vrf_pre_output(
		&self,
		key_type: KeyTypeId,
		public: &sr25519::Public,
		input: &sr25519::vrf::VrfInput,
	) -> Result<Option<sr25519::vrf::VrfPreOutput>, Error> {
		self.0.sr25519_vrf_pre_output(key_type, public, input)
	}

	fn ed25519_public_keys(&self, key_type: KeyTypeId) -> Vec<ed25519::Public> {
		self.0.ed25519_public_keys(key_type)
	}

	fn ed25519_generate_new(
		&self,
		key_type: KeyTypeId,
		seed: Option<&str>,
	) -> Result<ed25519::Public, Error> {
		self.0.ed25519_generate_new(key_type, seed)
	}

	fn ed25519_sign(
		&self,
		key_type: KeyTypeId,
		public: &ed25519::Public,
		msg: &[u8],
	) -> Result<Option<ed25519::Signature>, Error> {
		self.0.ed25519_sign(key_type, public, msg)
	}

	fn ecdsa_public_keys(&self, key_type: KeyTypeId) -> Vec<ecdsa::Public> {
		self.0.ecdsa_public_keys(key_type)
	}

	fn ecdsa_generate_new(
		&self,
		key_type: KeyTypeId,
		seed: Option<&str>,
	) -> Result<ecdsa::Public, Error> {
		self.0.ecdsa_generate_new(key_type, seed)
	}

	fn ecdsa_sign(
		&self,
		key_type: KeyTypeId,
		public: &ecdsa::Public,
		msg: &[u8],
	) -> Result<Option<ecdsa::Signature>, Error> {
		self.0.ecdsa_sign(key_type, public, msg)
	}

	fn ecdsa_sign_prehashed(
		&self,
		key_type: KeyTypeId,
		public: &ecdsa::Public,
		msg: &[u8; 32],
	) -> Result<Option<ecdsa::Signature>, Error> {
		self.0.ecdsa_sign_prehashed(key_type, public, msg)
	}

	/// Insert a new secret key.
	fn insert(&self, key_type: KeyTypeId, suri: &str, public: &[u8]) -> Result<(), ()> {
		self.0.insert(key_type, suri, public)
	}

	/// List all supported keys of a given type.
	///
	/// Returns a set of public keys the signer supports in raw format.
	fn keys(&self, key_type: KeyTypeId) -> Result<Vec<Vec<u8>>, Error> {
		self.0.keys(key_type)
	}

	/// Checks if the private keys for the given public key and key type
	/// combinations exist.
	///
	/// Returns `true` iff all private keys could be found.
	fn has_keys(&self, public_keys: &[(Vec<u8>, KeyTypeId)]) -> bool {
		dbg!("===================> USING THE SCHNORR KEYSTORE");
		for (key, key_id) in public_keys {
			dbg!("===================> {}", key_id);
			// dbg!("===================> {}", key);
			let pk_bytes: [u8; 32] = key.clone().try_into().expect("Invalid PK format");
			let grandpa_key = ed25519::Public::from_slice(key.as_slice()).expect("Failed to create aura key");
			dbg!(&grandpa_key);
			// dbg!("===================> {}", self.0.key_pair::<sp_application_crypto::ed25519::AppPair>(key.as_slice()).unwrap().is_some());

			let pk = Public(InnerPublicBytes::from_slice(&pk_bytes).expect("Failed to create key from slice"));
			dbg!(&pk);
			dbg!("===================> {}", self.0.key_pair::<Pair>(&pk).unwrap());

			let pk: Vec<u8> = <Public as AsRef<[u8]>>::as_ref(&pk).to_vec();
			let res = self.0.has_keys(&[(pk, key_id.clone())]);
			dbg!("Has keys (inner) {}", res);
		}
		let res = self.0.has_keys(public_keys);
		dbg!("Has keys {}", res);
		res
	}

	fn sign_with(
		&self,
		id: KeyTypeId,
		crypto_id: CryptoTypeId,
		public: &[u8],
		msg: &[u8],
	) -> Result<Option<Vec<u8>>, Error> {
		let signature = match crypto_id {
			CRYPTO_ID => {
				let pk_bytes: [u8; 32] = public.try_into().expect("Invalid PK format");
				let pk = Public(InnerPublicBytes::from_slice(&pk_bytes).expect("Failed to create key from slice"));
				if let Some(pair) = self.0.key_pair(&pk)? {
					dbg!("Producing signature");
					let shcnorr_sig: Vec<u8> =
						<Pair as TraitPair>::sign(&pair, msg).as_slice().to_vec();
					Some(shcnorr_sig)
				} else {
					dbg!("NOT Producing signature");
					None
				}
			},
			_ => self.0.sign_with(id, crypto_id, public, msg)?,
		};

		Ok(signature)
	}
}
