#![allow(non_snake_case)]
//! Implementation of the Schnorr signature scheme over the JubJub elliptic
//! curve, using Poseidon as the hash function.
//!
//! This crate provides the core cryptographic primitives needed to generate and
//! verify Schnorr signatures in zero-knowledge-friendly environments. By
//! leveraging the Poseidon hash, it is optimized for use in SNARK-based systems
//! where efficiency in constraint systems is critical.

use core::fmt::Debug;

use blstrs::{Fr, JubjubExtended, JubjubSubgroup as Point, Scalar};
use ff::Field;
use group::{Group, GroupEncoding};
use midnight_circuits::{
    ecc::curves::CircuitCurve, hash::poseidon::PoseidonChip, instructions::SpongeCPU,
};
use rand_core::{CryptoRng, RngCore};
use crate::poseidon::PoseidonError;

/// Poseidon hash function
pub type Poseidon = PoseidonChip<Scalar>;

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
            PoseidonError::NotCanonical => Self::InvalidMsgFormat
        }
    }
}

impl KeyPair {
    /// Returns the verifying key
    pub fn vk(&self) -> VerifyingKey {
        VerifyingKey(self.1)
    }

    /// Generate a Schnorr keypair from a random number generator.
    pub fn generate<R: RngCore>(rng: &mut R) -> Self {
        let sk = Fr::random(rng);
        let pk = Point::generator() * sk;
        Self(sk, pk)
    }

    /// Generates a Schnorr keypair from a seed.
    pub fn generate_from_seed(seed: [u8; 64]) -> Self {
        let sk = Fr::from_bytes_wide(&seed);
        let pk = Point::generator() * sk;
        Self(sk, pk)
    }

    /// Sign a message using this private key.
    ///
    /// # Arguments
    /// - `msg`: byte slice representing the message
    /// - `poseidon`: instance of the Poseidon hash function (preconfigured)
    ///
    /// # Returns
    /// A Schnorr `Signature`.
    pub fn sign(&self, msg: &[Scalar], rng: &mut (impl RngCore + CryptoRng)) -> SchnorrSignature {
        // Generate a random nonce
        let a = Fr::random(rng);
        let A = Point::generator() * a;

        // Compute challenge e = H(R || PK || msg)
        let c_input = [&to_coords(&A), &to_coords(&self.1), msg].concat();
        let c = hash_to_jj_scalar(&c_input);

        // Compute the response, r = a + c * sk
        let r = a + c * self.0;

        SchnorrSignature { A, r }
    }
}

impl SchnorrSignature {
    /// Verify a Schnorr signature.
    ///
    /// # Arguments
    /// - `msg`: byte slice of the original signed message
    /// - `sig`: the `Signature` object to verify
    /// - `poseidon`: Poseidon hash function instance
    ///
    /// # Error
    /// Function fails if the signature is not valid
    pub fn verify(&self, msg: &[Scalar], pk: &VerifyingKey) -> Result<(), SchnorrError> {
        let c_input = [&to_coords(&self.A), &to_coords(&pk.0), msg].concat();
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
        out[..32].copy_from_slice(&self.A.to_bytes());
        out[32..].copy_from_slice(&self.r.to_bytes());

        out
    }

    /// Converts a slice of bytes to a Signature
    ///
    /// # Error
    /// if the bytes do not represent a canonical `(Point, Scalar)` pair.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SchnorrError> {
        if bytes.len() != 64 {
            return Err(SchnorrError::InvalidSignatureFormat);
        }
        let A = Point::from_bytes(&bytes[..32].try_into().unwrap())
            .into_option()
            .ok_or(SchnorrError::InvalidSignatureFormat)?;
        let r = Fr::from_bytes(&bytes[32..].try_into().unwrap())
            .into_option()
            .ok_or(SchnorrError::InvalidSignatureFormat)?;

        Ok(Self { A, r })
    }
}

impl VerifyingKey {
    /// Converts a verifying key to a byte array.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Converts a slice of bytes to a VerifyingKey
    ///
    /// # Error
    /// if the bytes do not represent a canonical `Point` pair.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SchnorrError> {
        if bytes.len() != 32 {
            return Err(SchnorrError::InvalidPkFormat);
        }

        let pk = Point::from_bytes(&bytes[..32].try_into().unwrap())
            .into_option()
            .ok_or(SchnorrError::InvalidPkFormat)?;

        Ok(Self(pk))
    }
}

/// Helper function that converts a `JubJubSubgroup` point to its coordinates
fn to_coords(point: &Point) -> Vec<Scalar> {
    let extended_point: JubjubExtended = (*point).into();
    let coords = extended_point.coordinates().expect("Cannot be identity");

    vec![coords.0, coords.1]
}

/// Helper function that hashes into a JubJub scalar, by taking the mod
/// reduction of the output (which is in the base field, or BLS12-381's scalar
/// field).
fn hash_to_jj_scalar(input: &[Scalar]) -> Fr {
    let mut state = Poseidon::init(Some(input.len()));
    Poseidon::absorb(&mut state, input);
    let e = Poseidon::squeeze(&mut state);

    // Now we need to convert a BLS scalar to a JubJub scalar
    let mut bytes_wide = [0u8; 64];
    bytes_wide[..32].copy_from_slice(&e.to_bytes_le());

    Fr::from_bytes_wide(&bytes_wide)
}

#[cfg(test)]
mod tests {
    use rand_core::OsRng;

    use super::*;

    #[test]
    fn schnorr_jubjub() {
        let mut rng = OsRng;

        let signing_key = Pair::generate(&mut rng);
        let msg = Scalar::random(&mut rng);

        let sig = signing_key.sign(&[msg], &mut rng);

        assert!(sig.verify(&[msg], &signing_key.vk()).is_ok());
    }

    #[test]
    fn schnorr_jubjub_bytes() {
        let mut rng = OsRng;

        let signing_key = Pair::generate(&mut rng);
        let msg = Scalar::random(&mut rng).to_bytes_le();

        let sig = signing_key.sign(
            &SchnorrSignature::msg_from_bytes(&msg, true).unwrap(),
            &mut rng,
        );

        assert!(
            sig.verify(
                &SchnorrSignature::msg_from_bytes(&msg, true).unwrap(),
                &signing_key.vk()
            )
            .is_ok()
        );

        let mut msg = [0u8; 100];
        rng.fill_bytes(&mut msg);

        let sig = signing_key.sign(
            &SchnorrSignature::msg_from_bytes(&msg, false).unwrap(),
            &mut rng,
        );

        assert!(
            sig.verify(
                &SchnorrSignature::msg_from_bytes(&msg, false).unwrap(),
                &signing_key.vk()
            )
            .is_ok()
        );
    }
}
