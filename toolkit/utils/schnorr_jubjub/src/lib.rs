#![cfg_attr(not(feature = "std"), no_std)]
//! This crate implements the Schnorr signature scheme over the JubJub elliptic
//! curve, using the Poseidon hash function defined over the JubJub base field.
//!
//! It is intended exclusively for use within the BEEFY protocol.

extern crate alloc;

mod beefy_structures;
// mod keystore;
mod poseidon;
mod primitive;
mod runtime;

pub use beefy_structures::{InnerPublicBytes, Public, Signature};

pub use poseidon::{PoseidonJubjub,};
