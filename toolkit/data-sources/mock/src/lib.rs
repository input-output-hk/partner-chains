//! # Partner Chains Mocked Data Sources
//!
//! This crate provides mocked implementations of all data source interfaces used
//! by the Partner Chain toolkit's components, that can be wired into a Substrate
//! node to avoid having access to a real data source like the Db-Sync data sources
//! provided by `partner_chain_db_sync_data_sources` crate.
//!
//! # Usage
//!
//! *Important*: The mock data sources completely replace any interaction of the Partner
//!              Chain with its Cardano main chain, making them suitable only for local
//!              development and early exploration. They should not be used in production
//!              of public testnets.
//!
//! The mock data sources defined in this crate are meant to make it possible to run
//! a Partner Chain node with as little additional work as possible. Because of that,
//! most of them return either empty or constant data. The important exception is the
//! [AuthoritySelectionDataSourceMock] which can be fully configured with epoch data.
//! See each data source's documentation to see if and how it can be configured.
//!
//!
//! The mock data sources should be created as part of the node code, like so:
//! ```rust
//! use partner_chains_mock_data_sources::*;
//! use std::error::Error;
//! use std::sync::Arc;
//!
//! pub fn create_mock_data_sources()
//! -> std::result::Result<(), Box<dyn Error + Send + Sync + 'static>> {
//!    /// The block data source is reused by other data sources
//!    let block = Arc::new(BlockDataSourceMock::new_from_env()?);
//!
//!    let mc_hash             = Arc::new(McHashDataSourceMock::new(block.clone()));
//!    let sidechain_rpc       = Arc::new(SidechainRpcDataSourceMock::new(block));
//!    let authority_selection = Arc::new(AuthoritySelectionDataSourceMock::new_from_env()?);
//!    let native_token        = Arc::new(NativeTokenDataSourceMock::new());
//!    let block_participation = Arc::new(StakeDistributionDataSourceMock::new());
//!    let governed_map        = Arc::new(GovernedMapDataSourceMock::default());
//!
//!    Ok(())
//! }
//! ```
//!
//! After that they can be passed as dependencies to other Partner Chains toolkit components.

#![deny(missing_docs)]

#[cfg(feature = "block-source")]
mod block;
#[cfg(feature = "block-source")]
pub use block::BlockDataSourceMock;

#[cfg(feature = "candidate-source")]
mod candidate;
#[cfg(feature = "candidate-source")]
pub use candidate::{AuthoritySelectionDataSourceMock, MockRegistrationsConfig};

#[cfg(feature = "governed-map")]
mod governed_map;
#[cfg(feature = "governed-map")]
pub use governed_map::GovernedMapDataSourceMock;

#[cfg(feature = "mc-hash")]
mod mc_hash;
#[cfg(feature = "mc-hash")]
pub use mc_hash::McHashDataSourceMock;

#[cfg(feature = "native-token")]
mod native_token;
#[cfg(feature = "native-token")]
pub use native_token::NativeTokenDataSourceMock;

#[cfg(feature = "sidechain-rpc")]
mod sidechain_rpc;
#[cfg(feature = "sidechain-rpc")]
pub use sidechain_rpc::SidechainRpcDataSourceMock;

#[cfg(feature = "block-participation")]
mod stake_distribution;
#[cfg(feature = "block-participation")]
pub use stake_distribution::StakeDistributionDataSourceMock;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
