//! Off-chain code for Partner Chains Smart Contracts and general purpose utilities related to Cardano

/// Primitives used for awaiting for tx being observed on the blockchain
pub mod await_tx;
/// Collateral selection algorithm
pub mod collateral_selection;
/// General purpose code for interacting with cardano-serialization-lib
pub mod csl;
/// Supports D-Parameter upsert
pub mod d_param;
/// Supports governance initialization
pub mod init_governance;
#[cfg(test)]
pub mod ogmios_mock;
/// Supports Permissioned Candidates upsert
pub mod permissioned_candidates;
/// Utilities for handling Plutus script data
mod plutus_script;
/// Supports candidate registration
mod register;
/// Provides synthetized scripts data
pub mod scripts_data;
#[cfg(test)]
pub mod test_values;
/// Module for interaction with the untyped plutus scripts
pub mod untyped_plutus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OffchainError {
	OgmiosError(String),
	InternalError(String),
}
