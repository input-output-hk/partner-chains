//! # CLI Commands for Partner Chains
//!
//! This crate provides command structures and execution logic for Partner Chains operations.
//! These commands provide cryptographic operations, signature generation, and blockchain
//! interaction capabilities.
//!
//! ## Core Functionality
//!
//! - **Registration Signatures**: Generate cryptographic signatures for validator registration
//! - **Address Association**: Create signatures linking Cardano and Partner Chain addresses
//! - **Block Producer Metadata**: Sign metadata for block producer operations
//! - **Genesis UTXO Retrieval**: Query the genesis UTXO from on-chain storage
//! - **Key Parameter Handling**: Parse and validate various cryptographic key formats
//!
//! ## Integration
//!
//! These commands are exposed through the Partner Chains node CLI via the
//! `PartnerChainsSubcommand` enum in the `partner-chains-node-commands` crate.
//! Each command implements the clap `Parser` trait for argument parsing and provides
//! an `execute` method for performing the required operations.
//!
//! ## Architecture
//!
//! Commands follow a consistent pattern:
//! - Struct fields represent command parameters with clap annotations
//! - `execute` methods perform the core logic and output results
//! - Key parameter types provide secure parsing and validation
//! - Output structures serialize results as JSON for consumption

pub mod address_association_signatures;
pub mod block_producer_metadata_signatures;
pub mod get_genesis_utxo;
pub mod key_params;
pub mod registration_signatures;
