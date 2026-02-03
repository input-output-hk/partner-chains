//! A fresh FRAME-based Substrate node, ready for hacking.
#![allow(deprecated)]

pub mod chain_spec;
mod data_sources;
mod inherent_data;
mod mc_hash_block_import;
pub mod rpc;
pub mod service;
pub mod staging;
pub mod template_chain_spec;
pub mod testnet;

#[cfg(test)]
mod tests;
