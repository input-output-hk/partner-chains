//! Core API for the main chain queries

use plutus::Datum;
use sidechain_domain::*;
#[allow(unused_imports)]
use std::sync::Arc;
use thiserror::Error;

/// Types that will be used by the Cardano follower
pub mod common;

#[cfg(feature = "block-source")]
pub mod block;
#[cfg(feature = "block-source")]
pub use block::BlockDataSource;

#[cfg(feature = "candidate-source")]
pub mod candidate;
#[cfg(feature = "candidate-source")]
pub use candidate::CandidateDataSource;

#[cfg(feature = "native-token")]
pub mod native_token;
#[cfg(feature = "native-token")]
pub use native_token::NativeTokenManagementDataSource;

#[cfg(feature = "std")]
pub mod mock_services;

#[derive(Debug, PartialEq, Error)]
pub enum DataSourceError {
	#[error("Bad request: `{0}`.")]
	BadRequest(String),
	#[error("Internal error of data source: `{0}`.")]
	InternalDataSourceError(String),
	#[error("Could not decode {datum:?} to {to:?}, this means that there is an error in Plutus scripts or chain-follower is obsolete.")]
	DatumDecodeError { datum: Datum, to: String },
	#[error("'{0}' not found. Possible causes: main chain follower configuration error, db-sync not synced fully, or data not set on the main chain.")]
	ExpectedDataNotFound(String),
	#[error("Invalid data. {0} Possible cause it an error in Plutus scripts or chain-follower is obsolete.")]
	InvalidData(String),
}

pub type Result<T> = std::result::Result<T, DataSourceError>;
