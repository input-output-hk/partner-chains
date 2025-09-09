//! Crate providing implementations of Partner Chain Data Sources that read from Db-Sync Postgres.
//!
//! # Usage
//!
//! ## Adding to the node
//!
//! All data sources defined in this crate require a Postgres connection pool [PgPool] to run
//! queries, which should be shared between all data sources. For convenience, this crate provides
//! a helper function [get_connection_from_env] that will create a connection pool based on
//! configuration read from node environment.
//!
//! Each data source also accepts an optional Prometheus metrics client [McFollowerMetrics] for
//! reporting metrics to the Substrate's Prometheus metrics service. This client can be obtained
//! using the [register_metrics_warn_errors] function.
//!
//! In addition to these two common arguments, some data sources depend on [BlockDataSourceImpl]
//! which provides basic queries about blocks, and additional configuration for their data cache
//! size.
//!
//! An example node code that creates the data sources can look like the following:
//!
//! ```rust
//! # use std::error::Error;
//! # use std::sync::Arc;
//! use partner_chains_db_sync_data_sources::*;
//!
//! pub const CANDIDATES_FOR_EPOCH_CACHE_SIZE: usize = 64;
//! pub const STAKE_CACHE_SIZE: usize = 100;
//! pub const GOVERNED_MAP_CACHE_SIZE: u16 = 100;
//!
//! async fn create_data_sources(
//!     metrics_registry_opt: Option<&substrate_prometheus_endpoint::Registry>
//! ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let metrics = register_metrics_warn_errors(metrics_registry_opt);
//!     let pool = get_connection_from_env().await?;
//!
//!     // Block data source is shared by others for cache reuse
//!     let block = Arc::new(BlockDataSourceImpl::new_from_env(pool.clone()).await?);
//!
//!     let sidechain_rpc = SidechainRpcDataSourceImpl::new(block.clone(), metrics.clone());
//!
//!     let mc_hash = Arc::new(McHashDataSourceImpl::new(block.clone(), metrics.clone()));
//!
//!     let authority_selection =
//!         CandidatesDataSourceImpl::new(pool.clone(), metrics.clone())
//!     	.await?
//!     	.cached(CANDIDATES_FOR_EPOCH_CACHE_SIZE)?;
//!
//!     let native_token =
//!         NativeTokenManagementDataSourceImpl::new_from_env(pool.clone(), metrics.clone()).await?;
//!
//!     let block_participation =
//!     	StakeDistributionDataSourceImpl::new(pool.clone(), metrics.clone(), STAKE_CACHE_SIZE);
//!
//!     let governed_map =
//!         GovernedMapDataSourceCachedImpl::new(pool, metrics.clone(), GOVERNED_MAP_CACHE_SIZE, block).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Cardano DB Sync configuration
//!
//! Partner Chains data sources require specific Db-Sync configuration to be set for them to
//! operate correctly:
//! - `insert_options.tx_out.value`: must be either `"enable"` (default) or `"consumed"`.
//!   The data sources in this crate that need to query transaction intputs automatically detect
//!   which option is used and adjust their queries accordingly. This requires the database to be
//!   already initialized by db-sync. When run for an uninitialized database, the data sources
//!   will default to the `"enable"` option.
//! - `insert_options.tx_out.use_address_table`: must be `false` (default).
//! - `insert_options.ledger`: must be `"enable"` (default).
//! - `insert_options.multi_asset`: must be `true` (default).
//! - `insert_options.governance`: must `"enable"` (default).
//! - `insert_options.remove_jsonb_from_schema`: must be `"disable"` (default).
//! - `insert_options.plutus`: must be `"enable"` (default).
//!
//! The default Cardano DB Sync configuration meets these requirements, so Partner Chain node
//! operators that do not wish to use any custom configuration can use the defaults, otherwise
//! they must preserve the values described above. See [Db-Sync configuration docs] for more
//! information.
//!
//! ## Custom Indexes
//!
//! In addition to indexes automatically created by Db-Sync itself, data sources in this crate
//! require additional ones to be created for some of the queries to execute efficiently. These
//! indexes are:
//! - `idx_ma_tx_out_ident ON ma_tx_out(ident)`
//! - `idx_tx_out_address ON tx_out USING hash (address)`
//!
//! The data sources in this crate automatically create these indexes when needed at node startup.
//!
//! [PgPool]: sqlx::PgPool
//! [BlockDataSourceImpl]: crate::block::BlockDataSourceImpl
//! [McFollowerMetrics]: crate::metrics::McFollowerMetrics
//! [get_connection_from_env]: crate::data_sources::get_connection_from_env
//! [register_metrics_warn_errors]: crate::metrics::register_metrics_warn_errors
//! [Db-Sync configuration docs]: https://github.com/IntersectMBO/cardano-db-sync/blob/master/doc/configuration.md
#![deny(missing_docs)]
#![allow(rustdoc::private_intra_doc_links)]

pub use crate::{
	data_sources::{ConnectionConfig, PgPool, get_connection_from_env},
	metrics::{McFollowerMetrics, register_metrics_warn_errors},
};

#[cfg(feature = "block-source")]
pub use crate::block::{BlockDataSourceImpl, DbSyncBlockDataSourceConfig};
#[cfg(feature = "candidate-source")]
pub use crate::candidates::CandidatesDataSourceImpl;
#[cfg(feature = "governed-map")]
pub use crate::governed_map::{GovernedMapDataSourceCachedImpl, GovernedMapDataSourceImpl};
#[cfg(feature = "mc-hash")]
pub use crate::mc_hash::McHashDataSourceImpl;
#[cfg(feature = "native-token")]
pub use crate::native_token::NativeTokenManagementDataSourceImpl;
#[cfg(feature = "sidechain-rpc")]
pub use crate::sidechain_rpc::SidechainRpcDataSourceImpl;
#[cfg(feature = "block-participation")]
pub use crate::stake_distribution::StakeDistributionDataSourceImpl;

mod data_sources;
mod db_datum;
mod db_model;
mod metrics;

#[cfg(feature = "block-source")]
mod block;
#[cfg(feature = "candidate-source")]
mod candidates;
#[cfg(feature = "governed-map")]
mod governed_map;
#[cfg(feature = "mc-hash")]
mod mc_hash;
#[cfg(feature = "native-token")]
mod native_token;
#[cfg(feature = "sidechain-rpc")]
mod sidechain_rpc;
#[cfg(feature = "block-participation")]
mod stake_distribution;

#[derive(Debug)]
/// Wrapper error type for [sqlx::Error]
pub struct SqlxError(sqlx::Error);

impl From<sqlx::Error> for SqlxError {
	fn from(value: sqlx::Error) -> Self {
		SqlxError(value)
	}
}

impl From<SqlxError> for DataSourceError {
	fn from(e: SqlxError) -> Self {
		DataSourceError::InternalDataSourceError(e.0.to_string())
	}
}

impl From<SqlxError> for Box<dyn std::error::Error + Send + Sync> {
	fn from(e: SqlxError) -> Self {
		e.0.into()
	}
}

/// Error type returned by Db-Sync based data sources
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum DataSourceError {
	/// Indicates that the Db-Sync database rejected a request as invalid
	#[error("Bad request: `{0}`.")]
	BadRequest(String),
	/// Indicates that an internal error occured when querying the Db-Sync database
	#[error("Internal error of data source: `{0}`.")]
	InternalDataSourceError(String),
	/// Indicates that expected data was not found when querying the Db-Sync database
	#[error(
		"'{0}' not found. Possible causes: data source configuration error, db-sync not synced fully, or data not set on the main chain."
	)]
	ExpectedDataNotFound(String),
	/// Indicates that data returned by the Db-Sync database is invalid
	#[error(
		"Invalid data. {0} Possible cause is an error in Plutus scripts or data source is outdated."
	)]
	InvalidData(String),
}

/// Result type used by Db-Sync data sources
pub(crate) type Result<T> = std::result::Result<T, DataSourceError>;

#[cfg(test)]
mod tests {
	use ctor::{ctor, dtor};
	use std::sync::{OnceLock, mpsc};
	use testcontainers_modules::postgres::Postgres;
	use testcontainers_modules::testcontainers::{
		Container, ImageExt,
		bollard::query_parameters::{RemoveContainerOptions, StopContainerOptions},
		core::client::docker_client_instance,
		runners::SyncRunner,
	};

	static POSTGRES: OnceLock<Container<Postgres>> = OnceLock::new();

	fn init_postgres() -> Container<Postgres> {
		Postgres::default().with_tag("17.2").start().unwrap()
	}

	#[ctor]
	fn on_startup() {
		let postgres = POSTGRES.get_or_init(init_postgres);
		let database_url = &format!(
			"postgres://postgres:postgres@127.0.0.1:{}/postgres",
			postgres.get_host_port_ipv4(5432).unwrap()
		);
		// Needed for sqlx::test macro annotation
		unsafe {
			std::env::set_var("DATABASE_URL", database_url);
		}
	}

	#[dtor]
	fn on_shutdown() {
		let (tx, rx) = mpsc::channel();
		std::thread::spawn(move || {
			let runtime =
				tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
			runtime.block_on(async {
				let docker = docker_client_instance().await.unwrap();
				let id = POSTGRES.get().unwrap().id();
				docker.stop_container(id, None::<StopContainerOptions>).await.unwrap();
				docker.remove_container(id, None::<RemoveContainerOptions>).await.unwrap();
				tx.send(());
			});
		});
		let _: () = rx.recv().unwrap();
	}
}
