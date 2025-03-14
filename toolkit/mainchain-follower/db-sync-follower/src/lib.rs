//! Provides implementations of Data Sources that read from db-sync postgres
use cardano_serialization_lib::PlutusData;

pub mod data_sources;
mod db_datum;
pub mod db_model;
pub mod metrics;

// pub use db_model::{
// 	get_block_by_hash, get_latest_block_info, get_native_token_transfers,
// 	get_total_native_tokens_transfered, Asset, Block, BlockNumber, BlockTokenAmount,
// };
#[cfg(feature = "block-source")]
pub mod block;
#[cfg(feature = "candidate-source")]
pub mod candidates;
#[cfg(feature = "mc-hash")]
pub mod mc_hash;
#[cfg(feature = "native-token")]
pub mod native_token;
#[cfg(feature = "sidechain-rpc")]
pub mod sidechain_rpc;

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
	#[error("Could not decode {datum:?} to {to:?}, this means that there is an error in Plutus scripts or chain-follower is obsolete.")]
	DatumDecodeError { datum: PlutusData, to: String },
	#[error("'{0}' not found. Possible causes: main chain follower configuration error, db-sync not synced fully, or data not set on the main chain.")]
	ExpectedDataNotFound(String),
	#[error("Invalid data. {0} Possible cause it an error in Plutus scripts or chain-follower is obsolete.")]
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
		std::env::set_var("DATABASE_URL", database_url);
	}

	#[dtor]
	fn on_shutdown() {
		POSTGRES.get().iter().for_each(|postgres| postgres.rm());
	}
}
