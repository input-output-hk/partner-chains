//! Off-chain code for Partner Chains Smart Contracts and general purpose utilities related to Cardano

pub mod assemble_tx;
/// Primitives used for awaiting for tx being observed on the blockchain
pub mod await_tx;
/// Parsing and wrapping of Cardano keys
pub mod cardano_keys;
/// General purpose code for interacting with cardano-serialization-lib
pub mod csl;
/// Supports D-Parameter upsert
pub mod d_param;
/// Governance data types
pub mod governance;
/// Supports Governed Map key-value pair store management
pub mod governed_map;
/// Supports governance initialization
pub mod init_governance;
/// Types and functions related to smart-contracts that support MultiSig governance
pub mod multisig;
#[cfg(test)]
mod ogmios_mock;
/// Supports Permissioned Candidates upsert
pub mod permissioned_candidates;
/// Utilities for handling Plutus script data
pub mod plutus_script;
/// Supports candidate registration
pub mod register;
pub mod reserve;
/// Provides synthesized scripts data
pub mod scripts_data;
/// Signing transactions
pub mod sign_tx;
#[cfg(test)]
mod test_values;
/// Supports governance updates
pub mod update_governance;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OffchainError {
	OgmiosError(String),
	InternalError(String),
}
