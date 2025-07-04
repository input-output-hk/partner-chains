#![cfg_attr(not(feature = "std"), no_std)]
//! This crate implements the Schnorr signature scheme over the JubJub elliptic
//! curve, using the Poseidon hash function defined over the JubJub base field.
//!
//! It is intended exclusively for use within the BEEFY protocol.

extern crate alloc;

mod beefy_structures;

#[cfg(feature = "std")]
mod keystore;
mod poseidon;
mod primitive;
mod runtime;

pub use beefy_structures::{InnerPublicBytes, Public, Signature};

#[cfg(feature = "std")]
pub use keystore::SchnorrKeystore;
pub use poseidon::PoseidonJubjub;
