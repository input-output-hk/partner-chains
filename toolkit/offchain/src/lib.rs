/// General purpose code for interacting with cardano-serialization-lib
pub mod csl;
/// Provides synthetized scripts data
pub mod scripts_data;
/// Module for interaction with the untyped plutus scripts
pub mod untyped_plutus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OffchainError {
	OgmiosError(String),
	InternalError(String),
}
