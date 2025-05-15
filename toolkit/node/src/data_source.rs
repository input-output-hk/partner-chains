use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionDataSource;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use partner_chains_db_sync_data_sources::{
	block::BlockDataSourceImpl, candidates::CandidatesDataSourceImpl,
	mc_hash::McHashDataSourceImpl, metrics::register_metrics_warn_errors,
	sidechain_rpc::SidechainRpcDataSourceImpl,
};
use partner_chains_mock_data_sources::{
	block::BlockDataSourceMock, candidate::AuthoritySelectionDataSourceMock,
	mc_hash::McHashDataSourceMock, sidechain_rpc::SidechainRpcDataSourceMock,
};
use sidechain_mc_hash::McHashDataSource;
use std::sync::Arc;
use substrate_prometheus_endpoint::Registry;

#[cfg(feature = "block-participation")]
use {
	partner_chains_db_sync_data_sources::stake_distribution::StakeDistributionDataSourceImpl,
	partner_chains_mock_data_sources::stake_distribution::StakeDistributionDataSourceMock,
	sp_block_participation::inherent_data::BlockParticipationDataSource,
};

#[cfg(feature = "governed-map")]
use {
	partner_chains_db_sync_data_sources::governed_map::GovernedMapDataSourceCachedImpl,
	partner_chains_mock_data_sources::governed_map::GovernedMapDataSourceMock,
	sp_governed_map::GovernedMapDataSource,
};

#[cfg(feature = "native-token-management")]
use {
	partner_chains_db_sync_data_sources::native_token::NativeTokenManagementDataSourceImpl,
	partner_chains_mock_data_sources::native_token::NativeTokenDataSourceMock,
	sp_native_token_management::NativeTokenManagementDataSource,
};

// use sc_service::error::Error as ServiceError;

#[derive(Clone)]
pub struct PartnerChainsDataSource {
	pub mc_hash: Arc<dyn McHashDataSource + Send + Sync>,
	pub authority_selection: Arc<dyn AuthoritySelectionDataSource + Send + Sync>,
	pub sidechain_rpc: Arc<dyn SidechainRpcDataSource + Send + Sync>,
	#[cfg(feature = "native-token-management")]
	pub native_token: Arc<dyn NativeTokenManagementDataSource + Send + Sync>,
	#[cfg(feature = "block-participation")]
	pub block_participation: Arc<dyn BlockParticipationDataSource + Send + Sync>,
	#[cfg(feature = "governed-map")]
	pub governed_map: Arc<dyn GovernedMapDataSource + Send + Sync>,
}

impl PartnerChainsDataSource {
	pub async fn new_db_sync_or_mock_from_env(
		metrics_registry_opt: Option<&Registry>,
		candidates_for_epoch_cache_size: usize,
		#[cfg(feature = "governed-map")] governed_map_cache_size: u16,
		#[cfg(feature = "block-participation")] stake_cache_size: usize,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
		let use_mock = std::env::var("USE_MOCK_DATA_SOURCES")
			.ok()
			.and_then(|v| v.parse::<bool>().ok())
			.unwrap_or(false);
		if use_mock {
			Self::new_mock_from_env().map_err(|err| {
				format!("Failed to create mock data sources: {err}. Check configuration.").into()
			})
		} else {
			Self::new_db_sync_from_env(
				metrics_registry_opt,
				candidates_for_epoch_cache_size,
				governed_map_cache_size,
				stake_cache_size,
			)
			.await
			.map_err(|err| format!("Failed to create db-sync data sources: {err}").into())
		}
	}

	pub fn new_mock_from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
		let block = Arc::new(BlockDataSourceMock::new_from_env()?);
		Ok(Self {
			sidechain_rpc: Arc::new(SidechainRpcDataSourceMock::new(block.clone())),
			mc_hash: Arc::new(McHashDataSourceMock::new(block)),
			authority_selection: Arc::new(AuthoritySelectionDataSourceMock::new_from_env()?),
			#[cfg(feature = "native-token-management")]
			native_token: Arc::new(NativeTokenDataSourceMock::new()),
			#[cfg(feature = "block-participation")]
			block_participation: Arc::new(StakeDistributionDataSourceMock::new()),
			#[cfg(feature = "governed-map")]
			governed_map: Arc::new(GovernedMapDataSourceMock::default()),
		})
	}

	pub async fn new_db_sync_from_env(
		metrics_registry_opt: Option<&Registry>,
		candidates_for_epoch_cache_size: usize,
		#[cfg(feature = "governed-map")] governed_map_cache_size: u16,
		#[cfg(feature = "block-participation")] stake_cache_size: usize,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync + 'static>> {
		let metrics_opt = register_metrics_warn_errors(metrics_registry_opt);
		let pool =
			partner_chains_db_sync_data_sources::data_sources::get_connection_from_env().await?;
		// block data source is reused between mc_hash and sidechain_rpc to share cache
		let block = Arc::new(BlockDataSourceImpl::new_from_env(pool.clone()).await?);
		Ok(Self {
			sidechain_rpc: Arc::new(SidechainRpcDataSourceImpl::new(
				block.clone(),
				metrics_opt.clone(),
			)),
			mc_hash: Arc::new(McHashDataSourceImpl::new(block.clone(), metrics_opt.clone())),
			authority_selection: Arc::new(
				CandidatesDataSourceImpl::new(pool.clone(), metrics_opt.clone())
					.await?
					.cached(candidates_for_epoch_cache_size)?,
			),
			#[cfg(feature = "native-token-management")]
			native_token: Arc::new(
				NativeTokenManagementDataSourceImpl::new_from_env(
					pool.clone(),
					metrics_opt.clone(),
				)
				.await?,
			),
			#[cfg(feature = "block-participation")]
			block_participation: Arc::new(StakeDistributionDataSourceImpl::new(
				pool.clone(),
				metrics_opt.clone(),
				stake_cache_size,
			)),
			#[cfg(feature = "governed-map")]
			governed_map: Arc::new(
				GovernedMapDataSourceCachedImpl::new(
					pool,
					metrics_opt.clone(),
					governed_map_cache_size,
					block,
				)
				.await?,
			),
		})
	}
}
