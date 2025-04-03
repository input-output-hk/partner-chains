//! A fresh FRAME-based Substrate node, ready for hacking.

pub mod chain_spec;
mod data_sources;
mod inherent_data;
pub mod rpc;
pub mod service;
pub mod staging;
pub mod template_chain_spec;
pub mod testnet;

#[cfg(test)]
mod tests;
