//! # Selection algorithms
//!
//! This crate contains implementations of various algorithms for random selection of block-producer
//! committees used by the Partner Chains Toolkit.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

/// Random selection out of two pools of weighted (trustless) and unweighted (permissioned) candidates
/// controlled by a T/P ratio (D-parameter)
pub mod ariadne;
/// Random selection out of two pools of weighted (trustless) and unweighted (permissioned) candidates
/// controlled by a T/P ratio (D-parameter), with guaranteed seat allocations
pub mod ariadne_v2;
/// Simple independent weighted random selection
pub mod weighted_random;

#[cfg(test)]
mod tests;

/// Weight of individual candidate
pub type Weight = u128;
