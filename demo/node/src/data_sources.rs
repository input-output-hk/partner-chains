use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionDataSource;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use partner_chains_db_sync_data_sources::{
	BlockDataSourceImpl, CandidatesDataSourceImpl, GovernedMapDataSourceCachedImpl,
	McFollowerMetrics, McHashDataSourceImpl, NativeTokenManagementDataSourceImpl,
	SidechainRpcDataSourceImpl, StakeDistributionDataSourceImpl,
};
use partner_chains_mock_data_sources::{
	AuthoritySelectionDataSourceMock, BlockDataSourceMock, GovernedMapDataSourceMock,
	McHashDataSourceMock, NativeTokenDataSourceMock, SidechainRpcDataSourceMock,
	StakeDistributionDataSourceMock,
};
use sc_service::error::Error as ServiceError;
use sidechain_mc_hash::McHashDataSource;
use sp_block_participation::inherent_data::BlockParticipationDataSource;
use sp_governed_map::GovernedMapDataSource;
use sp_native_token_management::NativeTokenManagementDataSource;
use std::{error::Error, sync::Arc};

#[derive(Clone)]
pub struct DataSources {
	pub mc_hash: Arc<dyn McHashDataSource + Send + Sync>,
	pub authority_selection: Arc<dyn AuthoritySelectionDataSource + Send + Sync>,
	pub native_token: Arc<dyn NativeTokenManagementDataSource + Send + Sync>,
	pub sidechain_rpc: Arc<dyn SidechainRpcDataSource + Send + Sync>,
	pub block_participation: Arc<dyn BlockParticipationDataSource + Send + Sync>,
	pub governed_map: Arc<dyn GovernedMapDataSource + Send + Sync>,
}

pub(crate) async fn create_cached_data_sources(
	metrics_opt: Option<McFollowerMetrics>,
) -> std::result::Result<DataSources, ServiceError> {
	if use_mock_follower() {
		create_mock_data_sources().map_err(|err| {
			ServiceError::Application(
				format!("Failed to create mock data sources: {err}. Check configuration.").into(),
			)
		})
	} else {
		create_cached_db_sync_data_sources(metrics_opt).await.map_err(|err| {
			ServiceError::Application(
				format!("Failed to create db-sync data sources: {err}").into(),
			)
		})
	}
}

fn use_mock_follower() -> bool {
	std::env::var("USE_MOCK_DATA_SOURCES")
		.ok()
		.and_then(|v| v.parse::<bool>().ok())
		.unwrap_or(false)
}

pub fn create_mock_data_sources()
-> std::result::Result<DataSources, Box<dyn Error + Send + Sync + 'static>> {
	let block = Arc::new(BlockDataSourceMock::new_from_env()?);
	Ok(DataSources {
		sidechain_rpc: Arc::new(SidechainRpcDataSourceMock::new(block.clone())),
		mc_hash: Arc::new(McHashDataSourceMock::new(block)),
		authority_selection: Arc::new(AuthoritySelectionDataSourceMock::new_from_env()?),
		native_token: Arc::new(NativeTokenDataSourceMock::new()),
		block_participation: Arc::new(StakeDistributionDataSourceMock::new()),
		governed_map: Arc::new(GovernedMapDataSourceMock::default()),
	})
}

pub const CANDIDATES_FOR_EPOCH_CACHE_SIZE: usize = 64;
pub const STAKE_CACHE_SIZE: usize = 100;
pub const GOVERNED_MAP_CACHE_SIZE: u16 = 100;
pub async fn create_cached_db_sync_data_sources(
	metrics_opt: Option<McFollowerMetrics>,
) -> Result<DataSources, Box<dyn Error + Send + Sync + 'static>> {
	let pool = partner_chains_db_sync_data_sources::get_connection_from_env().await?;
	// block data source is reused between mc_hash and sidechain_rpc to share cache
	let block = Arc::new(BlockDataSourceImpl::new_from_env(pool.clone()).await?);
	Ok(DataSources {
		sidechain_rpc: Arc::new(SidechainRpcDataSourceImpl::new(
			block.clone(),
			metrics_opt.clone(),
		)),
		mc_hash: Arc::new(McHashDataSourceImpl::new(block.clone(), metrics_opt.clone())),
		authority_selection: Arc::new(
			CandidatesDataSourceImpl::new(pool.clone(), metrics_opt.clone())
				.await?
				.cached(CANDIDATES_FOR_EPOCH_CACHE_SIZE)?,
		),
		native_token: Arc::new(
			NativeTokenManagementDataSourceImpl::new_from_env(pool.clone(), metrics_opt.clone())
				.await?,
		),
		block_participation: Arc::new(StakeDistributionDataSourceImpl::new(
			pool.clone(),
			metrics_opt.clone(),
			STAKE_CACHE_SIZE,
		)),
		governed_map: Arc::new(
			GovernedMapDataSourceCachedImpl::new(
				pool,
				metrics_opt.clone(),
				GOVERNED_MAP_CACHE_SIZE,
				block,
			)
			.await?,
		),
	})
}
