//! Implements traits and supporting structures to enable the use of Schnorr
//! signatures over JubJub in BEEFY.
//!
//! This module provides the necessary abstractions and integrations for
//! leveraging JubJub-based Schnorr signatures within the BEEFY consensus
//! protocol, ensuring compatibility with Substrate's runtime and cryptographic
//! infrastructure.

use alloc::{format, vec::Vec, string::String};
use core::fmt::{Debug, Display, Formatter};

use crate::poseidon::PoseidonJubjub;
use ark_ec::AffineRepr;
use ark_ed_on_bls12_381::{EdwardsAffine, Fr};
use ark_ff::fields::Field;
use ark_serialize::CanonicalSerialize;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use sha2::Digest;
use sp_consensus_beefy::{AuthorityIdBound, BeefyAuthorityId};
use sp_core::{
	ByteArray, Decode, DecodeWithMemTracking, DeriveJunction, Encode, MaxEncodedLen,
	Pair as TraitPair,
	crypto::{
		CryptoType, CryptoTypeId, Derive, DeriveError, KeyTypeId, PublicBytes, SecretStringError,
	},
	crypto_bytes::SignatureBytes,
};
use sp_runtime::{
	RuntimeAppPublic,
	app_crypto::{AppCrypto, AppPair, AppPublic, AppSignature},
	traits::Convert,
};

use sp_core::crypto::Ss58Codec;

use crate::primitive::{SchnorrSignature, VerifyingKey};

/// Constant to represent the primitive of Schnorr over JubJub
pub const CRYPTO_ID: CryptoTypeId = CryptoTypeId(*b"jubP");

/// Constant to represent the primitive of Schnorr over JubJub
pub const SCHNORR_KEYTYPE_ID: KeyTypeId = KeyTypeId(*b"jubP");

/// The byte length of secret key seed.
pub const SEED_SERIALIZED_SIZE: usize = 32;

/// The byte length of serialized public key.
pub const PUBLIC_SERIALIZED_SIZE: usize = 32;

/// The byte length of serialized signature.
pub const SIGNATURE_SERIALIZED_SIZE: usize = 64;

#[doc(hidden)]
pub struct SchnorrJubJubTag;

pub type InnerPublicBytes = PublicBytes<PUBLIC_SERIALIZED_SIZE, SchnorrJubJubTag>;

/// Schnorr over JubJub public key
#[derive(
	Clone,
	Eq,
	Hash,
	PartialEq,
	PartialOrd,
	Ord,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub struct Public(pub InnerPublicBytes);

impl Display for Public {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "0x{}", hex::encode(self.as_slice()))
	}
}

impl Convert<Public, Vec<u8>> for Public {
	fn convert(beefy_id: Public) -> Vec<u8> {
		beefy_id.as_slice().to_vec()
	}
}

impl Serialize for Public {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&self.to_ss58check())
	}
}

impl<'de> Deserialize<'de> for Public {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Public::from_ss58check(&String::deserialize(deserializer)?)
			.map_err(|e| de::Error::custom(format!("{:?}", e)))
		// let bytes: &[u8] = Deserialize::deserialize(deserializer)?;
		// let key = Self::try_from(bytes)
		// 	.map_err(|_| serde::de::Error::custom("invalid public key bytes"))?;
		// Ok(key)
	}
}

impl Debug for Public {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		let s = self.to_ss58check();
		write!(f, "{} ({}...)", sp_core::hexdisplay::HexDisplay::from(&self.as_ref()), &s[0..8])
	}
}

impl AuthorityIdBound for Public {
	type SignatureHasher = PoseidonJubjub;
	type BoundedSignature = Signature;
}

impl<'a> TryFrom<&'a [u8]> for Public {
	type Error =
		<PublicBytes<PUBLIC_SERIALIZED_SIZE, SchnorrJubJubTag> as TryFrom<&'a [u8]>>::Error;

	fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
		Ok(Self(PublicBytes::<PUBLIC_SERIALIZED_SIZE, SchnorrJubJubTag>::try_from(value)?))
	}
}

impl AsMut<[u8]> for Public {
	fn as_mut(&mut self) -> &mut [u8] {
		self.0.as_mut()
	}
}

impl AsRef<[u8]> for Public {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl ByteArray for Public {
	const LEN: usize = PUBLIC_SERIALIZED_SIZE;
}

impl Derive for Public {}

impl sp_core::Public for Public {}

impl CryptoType for Public {
	type Pair = crate::primitive::KeyPair;
}

impl AppCrypto for Public {
	const ID: KeyTypeId = SCHNORR_KEYTYPE_ID;
	const CRYPTO_ID: CryptoTypeId = CRYPTO_ID;
	type Public = Public;
	type Signature = Signature;
	type Pair = crate::primitive::KeyPair;
}

impl AsRef<Public> for Public {
	fn as_ref(&self) -> &Public {
		self
	}
}

impl AsMut<Public> for Public {
	fn as_mut(&mut self) -> &mut Public {
		self
	}
}

impl AppPublic for Public {
	type Generic = Public;
}

pub type InnerSignatureBytes = SignatureBytes<SIGNATURE_SERIALIZED_SIZE, SchnorrJubJubTag>;

/// Schnorr signature over JubJub
#[derive(
	Clone,
	Eq,
	Hash,
	PartialEq,
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub struct Signature(InnerSignatureBytes);

impl Debug for Signature {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "Public({:?})", self.as_slice())
	}
}

impl CryptoType for Signature {
	type Pair = crate::primitive::KeyPair;
}

impl ByteArray for Signature {
	const LEN: usize = SIGNATURE_SERIALIZED_SIZE;
}

impl AsRef<[u8]> for Signature {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

impl AsMut<[u8]> for Signature {
	fn as_mut(&mut self) -> &mut [u8] {
		self.0.as_mut()
	}
}

impl<'a> TryFrom<&'a [u8]> for Signature {
	type Error =
		<SignatureBytes<SIGNATURE_SERIALIZED_SIZE, SchnorrJubJubTag> as TryFrom<&'a [u8]>>::Error;

	fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
		Ok(Self(SignatureBytes::<SIGNATURE_SERIALIZED_SIZE, SchnorrJubJubTag>::try_from(value)?))
	}
}

impl AppCrypto for Signature {
	const ID: KeyTypeId = SCHNORR_KEYTYPE_ID;
	const CRYPTO_ID: CryptoTypeId = CRYPTO_ID;
	type Public = Public;
	type Signature = Signature;
	type Pair = crate::primitive::KeyPair;
}

impl AsRef<Signature> for Signature {
	fn as_ref(&self) -> &Signature {
		self
	}
}

impl AsMut<Signature> for Signature {
	fn as_mut(&mut self) -> &mut Signature {
		self
	}
}

impl AppSignature for Signature {
	type Generic = Signature;
}

impl sp_core::crypto::Signature for Signature {}

impl CryptoType for crate::primitive::KeyPair {
	type Pair = crate::primitive::KeyPair;
}

impl TraitPair for crate::primitive::KeyPair {
	type Public = Public;
	type Seed = Seed;
	type Signature = Signature;

	fn derive<Iter: Iterator<Item = DeriveJunction>>(
		&self,
		_path: Iter,
		_seed: Option<Self::Seed>,
	) -> Result<(Self, Option<Self::Seed>), DeriveError> {
		unimplemented!()
	}

	fn from_seed_slice(seed: &[u8]) -> Result<Self, SecretStringError> {
		let h = sha2::Sha512::digest(&seed);

		let secret = Fr::from_random_bytes(h.as_slice())
			.expect("Failed to deserialize random bytes. This is a bug.");
		Ok(Self { 0: secret, 1: (EdwardsAffine::generator() * &secret).into() })
	}

	#[cfg(feature = "full_crypto")]
	fn sign(&self, message: &[u8]) -> Self::Signature {
		let msg = PoseidonJubjub::msg_from_bytes(message, false)
			.expect("With flag set to false, this should not fail. Report a bug.");

		let shcnorr_sig = self.sign(&msg);

		let bytes = shcnorr_sig.to_bytes();

		Signature(SignatureBytes::from_raw(bytes))
	}

	fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
		let msg = PoseidonJubjub::msg_from_bytes(message.as_ref(), false)
			.expect("With flag set to false, this should not fail. Report a bug.");

		let sig = SchnorrSignature::from_bytes(sig.as_ref());
		let pubkey = VerifyingKey::from_bytes(pubkey.as_ref());

		match (sig, pubkey) {
			(Ok(sig), Ok(pubkey)) => sig.verify(&msg, &pubkey).is_ok(),
			(_, _) => false,
		}
	}

	fn public(&self) -> Self::Public {
		let mut writer = Vec::new();
		self.1
			.serialize_compressed(&mut writer)
			.expect("Serialisation should not fail - writer is big enough");
		let bytes: [u8; 32] = writer.try_into().unwrap();

		Public(PublicBytes::from(bytes))
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		let mut res = Vec::with_capacity(64);
		self.0.serialize_compressed(&mut res).expect("Failed to serialize.");
		self.1.serialize_compressed(&mut res).expect("Failed to serialize.");

		res
	}
}

impl AppCrypto for crate::primitive::KeyPair {
	const ID: KeyTypeId = SCHNORR_KEYTYPE_ID;
	const CRYPTO_ID: CryptoTypeId = CRYPTO_ID;
	type Public = Public;
	type Signature = Signature;
	type Pair = crate::primitive::KeyPair;
}

impl AsRef<crate::primitive::KeyPair> for crate::primitive::KeyPair {
	fn as_ref(&self) -> &crate::primitive::KeyPair {
		self
	}
}

impl AsMut<crate::primitive::KeyPair> for crate::primitive::KeyPair {
	fn as_mut(&mut self) -> &mut crate::primitive::KeyPair {
		self
	}
}

impl AppPair for crate::primitive::KeyPair {
	type Generic = crate::primitive::KeyPair;
}

/// The raw secret seed, which can be used to reconstruct the secret
/// [`crate::primitive::KeyPair`].
type Seed = [u8; SEED_SERIALIZED_SIZE];

impl BeefyAuthorityId<PoseidonJubjub> for Public {
	fn verify(&self, signature: &<Self as RuntimeAppPublic>::Signature, msg: &[u8]) -> bool {
		<crate::primitive::KeyPair as TraitPair>::verify(signature, msg, self)
	}
}
