//! Provides implementations of Data Sources that read from db-sync postgres.
//!
//! ## Cardano DB Sync configuration
//! Cardano DB Sync instance requires specific configuration to maintain schema and data required by Partner Chains.
//! See [configuration doc](https://github.com/IntersectMBO/cardano-db-sync/blob/master/doc/configuration.md).
//!
//! `tx_out.value` - if present then it has to be `"enable"` (default) or `"consumed"`.
//!  When `"consumed"` is used then `tx_out.force_tx_in` has to be `true`.
//!  Code in this module depends on `tx_in` table.
//!
//! `tx_out.use_address_table` - if present then it has to be `false` (default).
//!
//! `ledger` - if present then it has to be `"enable"` (default).
//!
//! `multi_asset` - if present then it has to be `true` (default).
//!
//! `governance` - if present then it has to be `"enable"` (default).
//!
//! `remove_jsonb_from_schema` - if present then it has to be `"disable"` (default).
//!
//! The default Cardano DB Sync configuration meets these requirements.
//! It is enough to not provide a custom configuration.
//!
//! ## Custom Indexes
//! Cardano DB Sync creates a number of indexes for its own purpose.
//! Queries used in this module depend on some of them to be executed efficiently.
//! What is more, additional indexes are required:
//! * `idx_ma_tx_out_ident ON ma_tx_out(ident)`
//! * `idx_tx_out_address ON tx_out USING hash (address)`
//! This module provides functionality to automatically create such indexes on the node startup.
use cardano_serialization_lib::PlutusData;

pub mod data_sources;
mod db_datum;
mod db_model;
pub mod metrics;

#[cfg(feature = "block-source")]
pub mod block;
#[cfg(feature = "candidate-source")]
pub mod candidates;
#[cfg(feature = "governed-map")]
pub mod governed_map;
#[cfg(feature = "mc-hash")]
pub mod mc_hash;
#[cfg(feature = "native-token")]
pub mod native_token;
#[cfg(feature = "sidechain-rpc")]
pub mod sidechain_rpc;
#[cfg(feature = "block-participation")]
pub mod stake_distribution;

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

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum DataSourceError {
	#[error("Bad request: `{0}`.")]
	BadRequest(String),
	#[error("Internal error of data source: `{0}`.")]
	InternalDataSourceError(String),
	#[error(
		"Could not decode {datum:?} to {to:?}, this means that there is an error in Plutus scripts or the data source is outdated."
	)]
	DatumDecodeError { datum: PlutusData, to: String },
	#[error(
		"'{0}' not found. Possible causes: data source configuration error, db-sync not synced fully, or data not set on the main chain."
	)]
	ExpectedDataNotFound(String),
	#[error(
		"Invalid data. {0} Possible cause is an error in Plutus scripts or data source is outdated."
	)]
	InvalidData(String),
}

pub type Result<T> = std::result::Result<T, DataSourceError>;

#[cfg(test)]
mod tests {
	use ctor::{ctor, dtor};
	use std::sync::OnceLock;
	use testcontainers_modules::testcontainers::Container;
	use testcontainers_modules::{postgres::Postgres as PostgresImg, testcontainers::clients::Cli};

	static POSTGRES: OnceLock<Container<PostgresImg>> = OnceLock::new();
	static CLI: OnceLock<Cli> = OnceLock::new();

	fn init_postgres() -> Container<'static, PostgresImg> {
		let docker = CLI.get_or_init(Cli::default);
		docker.run(PostgresImg::default())
	}

	#[ctor]
	fn on_startup() {
		let postgres = POSTGRES.get_or_init(init_postgres);
		let database_url = &format!(
			"postgres://postgres:postgres@127.0.0.1:{}/postgres",
			postgres.get_host_port_ipv4(5432)
		);
		// Needed for sqlx::test macro annotation
		unsafe {
			std::env::set_var("DATABASE_URL", database_url);
		}
	}

	#[dtor]
	fn on_shutdown() {
		POSTGRES.get().iter().for_each(|postgres| postgres.rm());
	}
}
