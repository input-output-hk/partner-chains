//! # CLI Commands
//!
//! Provides command-line interface utilities for Partner Chains cryptographic operations.
//! This crate contains modules for generating signatures, managing cryptographic keys,
//! and interacting with Partner Chain blockchain data.
//!
//! ## Overview
//!
//! The cli-commands crate supports core Partner Chains operations:
//!
//! - **Address Association**: Link Cardano stake addresses with Partner Chain addresses
//! - **Block Producer Metadata**: Sign metadata for block producer registration
//! - **Validator Registration**: Generate signatures for mainchain and sidechain validator registration
//! - **Genesis UTXO Retrieval**: Query genesis UTXO from blockchain storage
//! - **Key Management**: Handle various cryptographic key types and conversions
//!
//! ## Usage Examples
//!
//! ### Address Association Signature
//!
//! ```bash
//! partner-chains-cli address-association-signatures \
//!   --genesis-utxo 59104061ffa0d66f9ba0135d6fc6a884a395b10f8ae9cb276fc2c3bfdfedc260#1 \
//!   --partnerchain-address d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d \
//!   --signing-key d75c630516c33a66b11b3444a70b65083aeb21353bd919cc5e3daa02c9732a84
//! ```
//!
//! ### Block Producer Metadata Signature
//!
//! ```bash
//! partner-chains-cli block-producer-metadata-signature \
//!   --genesis-utxo 0101010101010101010101010101010101010101010101010101010101010101#0 \
//!   --metadata-file metadata.json \
//!   --cross-chain-signing-key cb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
//! ```
//!
//! ### Validator Registration Signatures
//!
//! ```bash
//! partner-chains-cli registration-signatures \
//!   --genesis-utxo e41c9b57841e582c207bb68d5e9736fb48c7af5f1ec29ade00692fa5e0e47efa#4 \
//!   --mainchain-signing-key 2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c \
//!   --sidechain-signing-key 02dbfc8b66c22f931a6647fd86db2fc073dd564b99837226a1bdfe7a99578854ec \
//!   --registration-utxo 8ea10040249ad3033ae7c4d4b69e0b2e2b50a90741b783491cb5ddf8ced0d861#4
//! ```
//!
//! ## Integration
//!
//! This crate integrates with the broader Partner Chains ecosystem:
//!
//! - Uses `sidechain-domain` for core types and cryptographic primitives
//! - Leverages Substrate runtime APIs for blockchain interaction
//! - Supports Cardano stake pool operations through Ed25519 signatures
//! - Enables cross-chain communication via ECDSA signatures

pub mod address_association_signatures;
pub mod block_producer_metadata_signatures;
pub mod get_genesis_utxo;
pub mod key_params;
pub mod registration_signatures;
