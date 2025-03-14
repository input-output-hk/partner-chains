//! Selection logic for Sidechain Validators

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod ariadne;
/// Module that contains the pseudorandom algorithm for calculating the Sidechain Authorities from the Authority Candidates
pub mod weighted_random;

pub use weighted_random::*;

#[cfg(test)]
mod tests;
