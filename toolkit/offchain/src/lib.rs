//! Off-chain code for Partner Chains Smart Contracts and general purpose utilities related to Cardano

/// General purpose code for interacting with cardano-serialization-lib
pub mod csl;
/// Supports D-Parameter upsert
pub mod d_param;
/// Provides synthetized scripts data
pub mod scripts_data;
/// Module for interaction with the untyped plutus scripts
pub mod untyped_plutus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OffchainError {
	OgmiosError(String),
	InternalError(String),
}
