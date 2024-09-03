use db_sync_follower::native_token::NativeTokenManagementDataSourceImpl;
use db_sync_follower::{
	block::{BlockDataSourceImpl, DbSyncBlockDataSourceConfig},
	candidates::{cached::CandidateDataSourceCached, CandidatesDataSourceImpl},
	metrics::McFollowerMetrics,
};
use main_chain_follower_api::{
	BlockDataSource, CandidateDataSource, NativeTokenManagementDataSource,
};
use main_chain_follower_mock::{
	block::BlockDataSourceMock, candidate::MockCandidateDataSource,
	native_token::NativeTokenDataSourceMock,
};
use sc_service::error::Error as ServiceError;
use std::error::Error;
use std::sync::Arc;

#[derive(Clone)]
pub struct DataSources {
	pub block: Arc<dyn BlockDataSource + Send + Sync>,
	pub candidate: Arc<dyn CandidateDataSource + Send + Sync>,
	pub native_token: Arc<dyn NativeTokenManagementDataSource + Send + Sync>,
}

pub(crate) async fn create_cached_main_chain_follower_data_sources(
	metrics_opt: Option<McFollowerMetrics>,
) -> std::result::Result<DataSources, ServiceError> {
	if use_mock_follower() {
		create_mock_data_sources().map_err(|err| {
			ServiceError::Application(
				format!("Failed to create main chain follower mock: {err}. Check configuration.")
					.into(),
			)
		})
	} else {
		create_cached_data_sources(metrics_opt).await.map_err(|err| {
			ServiceError::Application(
				format!("Failed to create db-sync main chain follower: {err}").into(),
			)
		})
	}
}

fn use_mock_follower() -> bool {
	std::env::var("USE_MAIN_CHAIN_FOLLOWER_MOCK")
		.ok()
		.and_then(|v| v.parse::<bool>().ok())
		.unwrap_or(false)
}

pub fn create_mock_data_sources(
) -> std::result::Result<DataSources, Box<dyn Error + Send + Sync + 'static>> {
	let mc_epoch_duration_millis: u32 = std::env::var("MC__EPOCH_DURATION_MILLIS")
		.ok()
		.and_then(|v| v.parse::<u32>().ok())
		.unwrap();
	let block_data_source_mock = BlockDataSourceMock::new(mc_epoch_duration_millis);
	Ok(DataSources {
		block: Arc::new(block_data_source_mock),
		candidate: Arc::new(MockCandidateDataSource::from_env()?),
		native_token: Arc::new(NativeTokenDataSourceMock::new()),
	})
}

pub const CANDIDATES_FOR_EPOCH_CACHE_SIZE: usize = 64;

pub async fn create_cached_data_sources(
	metrics_opt: Option<McFollowerMetrics>,
) -> Result<DataSources, Box<dyn Error + Send + Sync + 'static>> {
	let pool = db_sync_follower::data_sources::get_connection_from_env().await?;
	let mc_epoch_config = &db_sync_follower::data_sources::read_mc_epoch_config()?;
	Ok(DataSources {
		block: Arc::new(BlockDataSourceImpl::from_config(
			pool.clone(),
			DbSyncBlockDataSourceConfig::from_env()?,
			mc_epoch_config,
			metrics_opt.clone(),
		)),
		candidate: Arc::new(CandidateDataSourceCached::new_from_env(
			CandidatesDataSourceImpl::from_config(pool.clone(), metrics_opt.clone()).await?,
			CANDIDATES_FOR_EPOCH_CACHE_SIZE,
		)?),
		native_token: Arc::new(NativeTokenManagementDataSourceImpl { pool, metrics_opt }),
	})
}
