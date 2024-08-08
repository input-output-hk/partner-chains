//! Provides implementations of Data Sources that read from db-sync postgres
use main_chain_follower_api::DataSourceError;

pub mod data_sources;
mod db_datum;
mod db_model;
pub mod metrics;

#[cfg(feature = "block-source")]
pub mod block;
#[cfg(feature = "candidate-source")]
pub mod candidates;
#[cfg(feature = "native-token")]
pub mod native_token;

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
