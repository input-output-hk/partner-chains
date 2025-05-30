//! Helpers for configuring and creating a Postgres database connection
use figment::Figment;
use figment::providers::Env;
use serde::Deserialize;
#[cfg(feature = "block-source")]
use sidechain_domain::mainchain_epoch::MainchainEpochConfig;
pub use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::str::FromStr;

/// Reads Cardano main chain epoch configuration from the environment.
///
/// See documentation of [MainchainEpochConfig::read_from_env] for the list of environment variables read.
#[cfg(feature = "block-source")]
pub fn read_mc_epoch_config() -> Result<MainchainEpochConfig, Box<dyn Error + Send + Sync>> {
	Ok(MainchainEpochConfig::read_from_env()
		.map_err(|e| format!("Failed to read main chain config: {}", e))?)
}

/// Postgres connection config used when creating a [PgPool].
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionConfig {
	/// Postgres connection pool, eg. `postgres://postgres-user:postgres-password@db-sync-postgres-host:5432/db-sync-db`
	pub(crate) db_sync_postgres_connection_string: SecretString,
}

impl ConnectionConfig {
	/// Reads Postgres connection config from the environment
	pub fn from_env() -> Result<Self, Box<dyn Error + Send + Sync + 'static>> {
		let config: Self = Figment::new()
			.merge(Env::raw())
			.extract()
			.map_err(|e| format!("Failed to read postgres data source connection: {e}"))?;
		Ok(config)
	}
}

#[derive(Clone, serde::Deserialize, Default)]
pub(crate) struct SecretString(pub String);

impl Debug for SecretString {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "***")
	}
}

async fn get_connection(
	connection_string: &str,
	acquire_timeout: std::time::Duration,
) -> Result<PgPool, Box<dyn Error + Send + Sync + 'static>> {
	let connect_options = PgConnectOptions::from_str(connection_string)?;
	let pool = PgPoolOptions::new()
		.max_connections(5)
		.acquire_timeout(acquire_timeout)
		.connect_with(connect_options.clone())
		.await
		.map_err(|e| {
			PostgresConnectionError(
				connect_options.get_host().to_string(),
				connect_options.get_port(),
				connect_options.get_database().unwrap_or("cexplorer").to_string(),
				e.to_string(),
			)
			.to_string()
		})?;
	Ok(pool)
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Could not connect to database: postgres://***:***@{0}:{1}/{2}; error: {3}")]
struct PostgresConnectionError(String, u16, String, String);

/// Returns a Postgres connection pool constructed using configuration read from environment
///
/// # Environment variables read:
/// - `DB_SYNC_POSTGRES_CONNECTION_STRING`: Postgres connection pool, eg.
///   `postgres://postgres-user:postgres-password@db-sync-postgres-host:5432/db-sync-db`
pub async fn get_connection_from_env() -> Result<PgPool, Box<dyn Error + Send + Sync + 'static>> {
	let config = ConnectionConfig::from_env()?;
	get_connection(
		config.db_sync_postgres_connection_string.0.as_str(),
		std::time::Duration::from_secs(30),
	)
	.await
}

#[cfg(test)]
mod tests {
	use super::*;
	use sqlx::Error::PoolTimedOut;

	#[tokio::test]
	async fn display_passwordless_connection_string_on_connection_error() {
		let expected_connection_error = PostgresConnectionError(
			"localhost".to_string(),
			4432,
			"cexplorer_test".to_string(),
			PoolTimedOut.to_string(),
		);
		let test_connection_string = "postgres://postgres:randompsw@localhost:4432/cexplorer_test";
		let actual_connection_error =
			get_connection(test_connection_string, std::time::Duration::from_millis(1)).await;
		assert_eq!(
			expected_connection_error.to_string(),
			actual_connection_error.unwrap_err().to_string()
		);
	}
}
