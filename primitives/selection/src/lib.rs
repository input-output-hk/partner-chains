//! Selection logic for Sidechain Validators

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

/// Module that contains the pseudorandom algorithm for calculating the Sidechain Authorities from the Authority Candidates
pub mod weighted_random;

pub use weighted_random::*;

pub mod impls;

#[cfg(test)]
mod tests;
