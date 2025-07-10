#![allow(non_snake_case)]
//! Implementation of the Schnorr signature scheme over the JubJub elliptic
//! curve, using Poseidon as the hash function.
//!
//! This crate provides the core cryptographic primitives needed to generate and
//! verify Schnorr signatures in zero-knowledge-friendly environments. By
//! leveraging the Poseidon hash, it is optimized for use in SNARK-based systems
//! where efficiency in constraint systems is critical.

use alloc::vec;
use alloc::vec::Vec;
use core::fmt::Debug;
use sha2::Digest;

use crate::poseidon::{PoseidonError, PoseidonJubjub};
use alloc::str::FromStr;
use alloc::string::ToString;
use ark_ec::AffineRepr;
use ark_ed_on_bls12_381::{EdwardsAffine as Point, Fq as Scalar, Fr};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use num_bigint::BigUint;

/// A Schnorr private key is a scalar from the Jubjub scalar field.
#[derive(Clone, Debug)]
pub struct KeyPair(pub(crate) Fr, pub(crate) Point);

/// A Schnorr public key is a point on the Jubjub curve.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct VerifyingKey(Point);

/// A Schnorr signature contains the announcement (nonce commitment) `A` and the
/// signature response `r`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchnorrSignature {
	A: Point,
	r: Fr,
}

#[derive(Debug)]
/// Error type used in Schnorr signatures
pub enum SchnorrError {
	/// Error converting a message represented in bytes to its field
	/// representation.
	InvalidMsgFormat,
	/// Error converting bytes to a signature.
	InvalidSignatureFormat,
	/// Error converting bytes to a public key.
	InvalidPkFormat,
	/// Error verifying a signature.
	InvalidSignature,
}

impl From<PoseidonError> for SchnorrError {
	fn from(value: PoseidonError) -> Self {
		match value {
			PoseidonError::NotCanonical => Self::InvalidMsgFormat,
		}
	}
}

/// Helper function to reduce little endian bytes modulo the order
/// of `Fr`
pub(crate) fn mod_p(bytes: &[u8]) -> Fr {
	let biguint = BigUint::from_bytes_be(bytes);
	Fr::from_str(&biguint.to_string())
		.expect("Failed to reduce bytes modulo Fr::MODULUS. This is a bug.")
}

impl KeyPair {
	/// Returns the verifying key
	pub fn vk(&self) -> VerifyingKey {
		VerifyingKey(self.1)
	}

	/// Generates a Schnorr keypair from a seed.
	pub fn generate_from_seed(seed: &[u8]) -> Self {
		let hashed_seed = sha2::Sha512::digest(&seed);

		let sk = mod_p(hashed_seed.as_slice());
		let pk = Point::generator() * sk;
		Self(sk, pk.into())
	}

	/// Sign a message using this private key.
	pub fn sign(&self, msg: &[Scalar]) -> SchnorrSignature {
		let mut bytes_nonce = [0u8; 32];
		self.0
			.serialize_compressed(bytes_nonce.as_mut_slice())
			.expect("Failed to serialize.");

		for scalar in msg {
			scalar
				.serialize_compressed(bytes_nonce.as_mut_slice())
				.expect("Failed to serialize.");
		}

		// Generate a random nonce
		// TODO: We compute it deterministically (as done in ed25519) to avoid needing a RNG
		let h = sha2::Sha512::digest(&bytes_nonce);

		let a = mod_p(h.as_slice());
		let A = (Point::generator() * a).into();

		// Compute challenge e = H(R || PK || msg)
		let c_input = [
			&to_coords(&A).expect("Shouldn't produce a signature with nonce = 0."),
			&to_coords(&self.1).expect("Your verifying key is the identity! This is a bug."),
			msg,
		]
		.concat();
		let c = hash_to_jj_scalar(&c_input);

		// Compute the response, r = a + c * sk
		let r = a + c * self.0;

		SchnorrSignature { A, r }
	}
}

impl SchnorrSignature {
	/// Verify a Schnorr signature.
	///
	/// # Error
	/// Function fails if the signature is not valid
	pub fn verify(&self, msg: &[Scalar], pk: &VerifyingKey) -> Result<(), SchnorrError> {
		let c_input = [
			&to_coords(&self.A).ok_or(SchnorrError::InvalidSignature)?,
			&to_coords(&pk.0).ok_or(SchnorrError::InvalidSignature)?,
			msg,
		]
		.concat();

		let c = hash_to_jj_scalar(&c_input);

		if Point::generator() * self.r == self.A + pk.0 * c {
			Ok(())
		} else {
			Err(SchnorrError::InvalidSignature)
		}
	}
}

impl SchnorrSignature {
	/// Converts a signature to a byte array.
	pub fn to_bytes(&self) -> [u8; 64] {
		let mut out = [0u8; 64];
		self.A.serialize_compressed(out.as_mut_slice()).expect("Failed to serialize.");
		self.r.serialize_compressed(&mut out[32..]).expect("Failed to serialize.");

		out
	}

	/// Converts a slice of bytes to a Signature
	///
	/// # Error
	/// if the bytes do not represent a canonical `(Point, Scalar)` pair.
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, SchnorrError> {
		let A = Point::deserialize_compressed(&bytes[..32])
			.map_err(|_| SchnorrError::InvalidSignatureFormat)?;
		let r = Fr::deserialize_compressed(&bytes[32..])
			.map_err(|_| SchnorrError::InvalidSignatureFormat)?;

		Ok(Self { A, r })
	}
}

impl VerifyingKey {
	/// Converts a verifying key to a byte array.
	pub fn to_bytes(&self) -> [u8; 32] {
		let mut out = [0u8; 32];
		self.0.serialize_compressed(out.as_mut_slice()).expect("Failed to serialize");
		out
	}

	/// Converts a slice of bytes to a VerifyingKey
	///
	/// # Error
	/// if the bytes do not represent a canonical `Point` pair.
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, SchnorrError> {
		let pk = Point::deserialize_compressed(bytes).map_err(|_| SchnorrError::InvalidPkFormat)?;

		Ok(Self(pk))
	}
}

/// Helper function that converts a `JubJubSubgroup` point to its coordinates
fn to_coords(point: &Point) -> Option<Vec<Scalar>> {
	let (x, y) = point.xy()?;

	Some(vec![x, y])
}

/// Helper function that hashes into a JubJub scalar, by taking the mod
/// reduction of the output (which is in the base field, or BLS12-381's scalar
/// field).
fn hash_to_jj_scalar(input: &[Scalar]) -> Fr {
	let mut state = PoseidonJubjub::init(Some(input.len()));
	PoseidonJubjub::absorb(&mut state, input);
	let e = PoseidonJubjub::squeeze(&mut state);

	// Now we need to convert a BLS scalar to a JubJub scalar
	let mut bytes_wide = [0u8; 64];
	e.serialize_compressed(bytes_wide.as_mut_slice()).expect("Failed to serialize");

	mod_p(&bytes_wide)
}

#[cfg(test)]
mod tests {
	use ark_ff::UniformRand;
	use rand_core::{OsRng, RngCore};

	use super::*;

	#[test]
	fn schnorr_jubjub() {
		let mut rng = OsRng;
		let mut seed = [0u8; 32];
		rng.fill_bytes(&mut seed);

		let signing_key = KeyPair::generate_from_seed(&seed);
		let msg = Scalar::rand(&mut rng);

		let sig = signing_key.sign(&[msg]);

		assert!(sig.verify(&[msg], &signing_key.vk()).is_ok());
	}

	#[test]
	fn schnorr_jubjub_bytes() {
		let mut rng = OsRng;
		let mut seed = [0u8; 32];
		rng.fill_bytes(&mut seed);

		let signing_key = KeyPair::generate_from_seed(&seed);

		let mut msg = [0u8; 32];
		Scalar::rand(&mut rng).serialize_compressed(msg.as_mut_slice()).unwrap();
		let msg = PoseidonJubjub::msg_from_bytes(&msg, true).unwrap();

		let sig = signing_key.sign(&msg);

		assert!(sig.verify(&msg, &signing_key.vk()).is_ok());

		let mut msg = [0u8; 100];
		rng.fill_bytes(&mut msg);
		let msg = PoseidonJubjub::msg_from_bytes(&msg, false)
			.expect("With flag set to false, this should not fail. Report a bug.");

		let sig = signing_key.sign(&msg);

		assert!(sig.verify(&msg, &signing_key.vk()).is_ok());
	}

	#[test]
	fn serde() {
		let mut rng = OsRng;
		let mut seed = [0u8; 32];
		rng.fill_bytes(&mut seed);

		let signing_key = KeyPair::generate_from_seed(&seed);
		let msg = Scalar::rand(&mut rng);

		let sig = signing_key.sign(&[msg]);

		let vk = signing_key.vk();
		let ser_vk = vk.to_bytes();
		let deser_vk = VerifyingKey::from_bytes(&ser_vk).unwrap();

		assert!(sig.verify(&[msg], &deser_vk).is_ok());

		let ser_sig = sig.to_bytes();
		let deser_sig = SchnorrSignature::from_bytes(&ser_sig).unwrap();

		assert!(deser_sig.verify(&[msg], &vk).is_ok());
	}

	// Helper test to generate test-vectors
	// #[test]
	// fn print_data() {
	// 	let seed =
	// 		b"belt hurt material survey skate group illness health electric frown live sword";
	// 	println!("{:?}", hex::encode(seed));
	// 	let keypair = KeyPair::generate_from_seed(seed);
	//
	// 	let vk = keypair.vk();
	// 	println!("{}", hex::encode(vk.to_bytes()));
	//
	// 	// let ss58 = vk.to_ss58check();
	// 	// println!("{}", ss58);
	// }
}
