use authority_selection_inherents::AuthoritySelectionDataSource;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use partner_chains_data_source_metrics::McFollowerMetrics;
use sidechain_domain::*;
use sidechain_mc_hash::McHashDataSource;
use std::{error::Error, sync::Arc};

use crate::data_sources::*;

struct SidechainRpcDataSourceImplDiff {
	dolos: Arc<partner_chains_dolos_data_sources::SidechainRpcDataSourceImpl>,
	dbsync: Arc<partner_chains_db_sync_data_sources::SidechainRpcDataSourceImpl>,
}

#[async_trait::async_trait]
impl SidechainRpcDataSource for SidechainRpcDataSourceImplDiff {
	async fn get_latest_block_info(
		&self,
	) -> Result<sidechain_domain::MainchainBlock, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.get_latest_block_info().await?;
		let dolos_output = self.dolos.get_latest_block_info().await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> SidechainRpcDataSource::get_latest_block_info mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}
}

struct McHashDataSourceImplDiff {
	dolos: Arc<partner_chains_dolos_data_sources::McHashDataSourceImpl>,
	dbsync: Arc<partner_chains_db_sync_data_sources::McHashDataSourceImpl>,
}

#[async_trait::async_trait]
impl McHashDataSource for McHashDataSourceImplDiff {
	async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.get_latest_stable_block_for(reference_timestamp).await?;
		let dolos_output = self.dolos.get_latest_stable_block_for(reference_timestamp).await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> McHashDataSource::get_latest_stable_block_for mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}

	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: sp_timestamp::Timestamp,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.get_stable_block_for(hash.clone(), reference_timestamp).await?;
		let dolos_output = self.dolos.get_stable_block_for(hash, reference_timestamp).await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> McHashDataSource::get_stable_block_for mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}

	async fn get_block_by_hash(
		&self,
		hash: McBlockHash,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.get_block_by_hash(hash.clone()).await?;
		let dolos_output = self.dolos.get_block_by_hash(hash).await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> McHashDataSource::get_block_by_hash mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}
}

struct AuthoritySelectionDataSourceImplDiff {
	dolos: Arc<partner_chains_dolos_data_sources::AuthoritySelectionDataSourceImpl>,
	dbsync: Arc<partner_chains_db_sync_data_sources::CandidateDataSourceCached>,
}

#[async_trait::async_trait]
impl AuthoritySelectionDataSource for AuthoritySelectionDataSourceImplDiff {
	async fn get_ariadne_parameters(
		&self,
		epoch_number: McEpochNumber,
		d_parameter_policy: PolicyId,
		permissioned_candidate_policy: PolicyId,
	) -> Result<
		authority_selection_inherents::AriadneParameters,
		Box<dyn std::error::Error + Send + Sync>,
	> {
		let reference = self
			.dbsync
			.get_ariadne_parameters(
				epoch_number,
				d_parameter_policy.clone(),
				permissioned_candidate_policy.clone(),
			)
			.await?;
		let dolos_output = self
			.dolos
			.get_ariadne_parameters(epoch_number, d_parameter_policy, permissioned_candidate_policy)
			.await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> McHashDataSource::get_ariadne_parameters mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}

	async fn get_candidates(
		&self,
		epoch_number: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self
			.dbsync
			.get_candidates(epoch_number, committee_candidate_address.clone())
			.await?;
		let dolos_output =
			self.dolos.get_candidates(epoch_number, committee_candidate_address).await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> McHashDataSource::get_candidates mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}

	async fn get_epoch_nonce(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<Option<EpochNonce>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.get_epoch_nonce(epoch_number).await?;
		let dolos_output = self.dolos.get_epoch_nonce(epoch_number).await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> McHashDataSource::get_epoch_nonce mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}

	async fn data_epoch(
		&self,
		for_epoch: McEpochNumber,
	) -> Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.data_epoch(for_epoch).await?;
		let dolos_output = self.dolos.data_epoch(for_epoch).await?;
		if reference != dolos_output {
			println!(
				">>>>>>>>>>>>>>>>>>>>>>>>>>>> McHashDataSource::data_epoch mismatch: dbs: {reference:?} dolos: {dolos_output:?}"
			)
		}
		Ok(reference)
	}
}

pub async fn create_diff_data_sources(
	metrics_opt: Option<McFollowerMetrics>,
) -> std::result::Result<DataSources, Box<dyn Error + Send + Sync + 'static>> {
	let dolos_client = partner_chains_dolos_data_sources::get_connection_from_env()?;
	let pool = partner_chains_db_sync_data_sources::get_connection_from_env().await?;
	let block_dbsync = Arc::new(
		partner_chains_db_sync_data_sources::BlockDataSourceImpl::new_from_env(pool.clone())
			.await?,
	);
	let block_dolos = Arc::new(
		partner_chains_dolos_data_sources::BlockDataSourceImpl::new_from_env(dolos_client.clone())
			.await?,
	);
	Ok(DataSources {
		sidechain_rpc: Arc::new(SidechainRpcDataSourceImplDiff {
			dolos: Arc::new(partner_chains_dolos_data_sources::SidechainRpcDataSourceImpl::new(
				dolos_client.clone(),
			)),
			dbsync: Arc::new(partner_chains_db_sync_data_sources::SidechainRpcDataSourceImpl::new(
				block_dbsync.clone(),
				metrics_opt.clone(),
			)),
		}),
		mc_hash: Arc::new(McHashDataSourceImplDiff {
			dolos: Arc::new(partner_chains_dolos_data_sources::McHashDataSourceImpl::new(
				block_dolos.clone(),
			)),
			dbsync: Arc::new(partner_chains_db_sync_data_sources::McHashDataSourceImpl::new(
				block_dbsync.clone(),
				metrics_opt.clone(),
			)),
		}),
		authority_selection: Arc::new(AuthoritySelectionDataSourceImplDiff {
			dolos: Arc::new(
				partner_chains_dolos_data_sources::AuthoritySelectionDataSourceImpl::new(
					dolos_client.clone(),
				),
			),
			dbsync: Arc::new(
				partner_chains_db_sync_data_sources::CandidatesDataSourceImpl::new(
					pool.clone(),
					metrics_opt.clone(),
				)
				.await?
				.cached(CANDIDATES_FOR_EPOCH_CACHE_SIZE)?,
			),
		}),
		block_participation: Arc::new(
			partner_chains_db_sync_data_sources::StakeDistributionDataSourceImpl::new(
				pool.clone(),
				metrics_opt.clone(),
				STAKE_CACHE_SIZE,
			),
		),
		governed_map: Arc::new(
			partner_chains_db_sync_data_sources::GovernedMapDataSourceCachedImpl::new(
				pool.clone(),
				metrics_opt.clone(),
				GOVERNED_MAP_CACHE_SIZE,
				block_dbsync.clone(),
			)
			.await?,
		),
		bridge: Arc::new(
			partner_chains_db_sync_data_sources::CachedTokenBridgeDataSourceImpl::new(
				pool,
				metrics_opt,
				block_dbsync,
				BRIDGE_TRANSFER_CACHE_LOOKAHEAD,
			),
		),
	})
}
