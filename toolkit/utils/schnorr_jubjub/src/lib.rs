//! This crate implements the Schnorr signature scheme over the JubJub elliptic
//! curve, using the Poseidon hash function defined over the JubJub base field.
//!
//! It is intended exclusively for use within the BEEFY protocol.

mod beefy_structures;
mod keystore;
mod primitive;
mod runtime;
mod poseidon;

pub use beefy_structures::{Public, InnerPublicBytes, Signature};
