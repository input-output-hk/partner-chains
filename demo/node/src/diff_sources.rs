use authority_selection_inherents::AuthoritySelectionDataSource;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use partner_chains_data_source_metrics::McFollowerMetrics;
use sidechain_domain::{byte_string::ByteString, *};
use sidechain_mc_hash::McHashDataSource;
use sp_block_participation::inherent_data::BlockParticipationDataSource;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
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
				"😱 Dolos source for SidechainRpcDataSource::get_latest_block_info returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
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
				"😱 Dolos source for McHashDataSource::get_latest_stable_block_for returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
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
				"😱 Dolos source for McHashDataSource::get_stable_block_for returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
	}

	async fn get_block_by_hash(
		&self,
		hash: McBlockHash,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.get_block_by_hash(hash.clone()).await?;
		let dolos_output = self.dolos.get_block_by_hash(hash).await?;
		if reference != dolos_output {
			println!(
				"😱 Dolos source for McHashDataSource::get_block_by_hash returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
	}
}

struct AuthoritySelectionDataSourceImplDiff {
	dolos: Arc<partner_chains_dolos_data_sources::AuthoritySelectionDataSourceImpl>,
	dbsync: Arc<partner_chains_db_sync_data_sources::CandidatesDataSourceImpl>,
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
				"😱 Dolos source for McHashDataSource::get_ariadne_parameters returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
	}

	async fn get_candidates(
		&self,
		epoch_number: McEpochNumber,
		committee_candidate_address: MainchainAddress,
	) -> Result<Vec<CandidateRegistrations>, Box<dyn std::error::Error + Send + Sync>> {
		let mut reference = self
			.dbsync
			.get_candidates(epoch_number, committee_candidate_address.clone())
			.await?;
		let mut dolos_output =
			self.dolos.get_candidates(epoch_number, committee_candidate_address).await?;
		reference.sort_by_key(|a| a.mainchain_pub_key().clone());
		dolos_output.sort_by_key(|a| a.mainchain_pub_key().clone());
		if reference != dolos_output {
			println!(
				"😱 Dolos source for McHashDataSource::get_candidates returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}\n"
			)
		}
		Ok(dolos_output)
	}

	async fn get_epoch_nonce(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<Option<EpochNonce>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.get_epoch_nonce(epoch_number).await?;
		let dolos_output = self.dolos.get_epoch_nonce(epoch_number).await?;
		if reference != dolos_output {
			println!(
				"😱 Dolos source for McHashDataSource::get_epoch_nonce returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}\n\n"
			)
		}
		Ok(dolos_output)
	}

	async fn data_epoch(
		&self,
		for_epoch: McEpochNumber,
	) -> Result<McEpochNumber, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self.dbsync.data_epoch(for_epoch).await?;
		let dolos_output = self.dolos.data_epoch(for_epoch).await?;
		if reference != dolos_output {
			println!(
				"😱 Dolos source for McHashDataSource::data_epoch returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
	}
}

struct StakeDistributionDataSourceImplDiff {
	dolos: Arc<partner_chains_dolos_data_sources::StakeDistributionDataSourceImpl>,
	dbsync: Arc<partner_chains_db_sync_data_sources::StakeDistributionDataSourceImpl>,
}

#[async_trait::async_trait]
impl BlockParticipationDataSource for StakeDistributionDataSourceImplDiff {
	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		epoch_number: McEpochNumber,
		pool_hashes: &[MainchainKeyHash],
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self
			.dbsync
			.get_stake_pool_delegation_distribution_for_pools(epoch_number, pool_hashes)
			.await?;
		let dolos_output = self
			.dolos
			.get_stake_pool_delegation_distribution_for_pools(epoch_number, pool_hashes)
			.await?;
		if reference.0 != dolos_output.0 {
			println!(
				"😱 Dolos source for BlockParticipationDataSource::get_stake_pool_delegation_distribution_for_pools returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
	}
}

struct GovernedMapDataSourceImplDiff {
	dolos: Arc<partner_chains_dolos_data_sources::GovernedMapDataSourceImpl>,
	dbsync: Arc<partner_chains_db_sync_data_sources::GovernedMapDataSourceImpl>,
}

#[async_trait::async_trait]
impl GovernedMapDataSource for GovernedMapDataSourceImplDiff {
	async fn get_state_at_block(
		&self,
		mc_block: McBlockHash,
		main_chain_scripts: MainChainScriptsV1,
	) -> Result<BTreeMap<String, ByteString>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self
			.dbsync
			.get_state_at_block(mc_block.clone(), main_chain_scripts.clone())
			.await?;
		let dolos_output = self.dolos.get_state_at_block(mc_block, main_chain_scripts).await?;
		if reference != dolos_output {
			println!(
				"😱 Dolos source for GovernedMapDataSource::get_state_at_block returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
	}

	async fn get_mapping_changes(
		&self,
		since_mc_block: Option<McBlockHash>,
		up_to_mc_block: McBlockHash,
		scripts: MainChainScriptsV1,
	) -> Result<Vec<(String, Option<ByteString>)>, Box<dyn std::error::Error + Send + Sync>> {
		let reference = self
			.dbsync
			.get_mapping_changes(since_mc_block.clone(), up_to_mc_block.clone(), scripts.clone())
			.await?;
		let dolos_output =
			self.dolos.get_mapping_changes(since_mc_block, up_to_mc_block, scripts).await?;
		if reference != dolos_output {
			println!(
				"😱 Dolos source for GovernedMapDataSource::get_mapping_changes returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
	}
}

struct TokenBridgeDataSourceImplDiff {
	dolos: Arc<partner_chains_dolos_data_sources::TokenBridgeDataSourceImpl>,
	dbsync: Arc<partner_chains_db_sync_data_sources::TokenBridgeDataSourceImpl>,
}

#[async_trait::async_trait]
impl<RecipientAddress: Send + Sync> TokenBridgeDataSource<RecipientAddress>
	for TokenBridgeDataSourceImplDiff
where
	RecipientAddress: std::fmt::Debug,
	RecipientAddress: Eq,
	RecipientAddress: (for<'a> TryFrom<&'a [u8]>),
{
	async fn get_transfers(
		&self,
		main_chain_scripts: MainChainScripts,
		data_checkpoint: BridgeDataCheckpoint,
		max_transfers: u32,
		current_mc_block_hash: McBlockHash,
	) -> Result<
		(Vec<BridgeTransferV1<RecipientAddress>>, BridgeDataCheckpoint),
		Box<dyn std::error::Error + Send + Sync>,
	> {
		let reference = self
			.dbsync
			.get_transfers(
				main_chain_scripts.clone(),
				data_checkpoint.clone(),
				max_transfers.clone(),
				current_mc_block_hash.clone(),
			)
			.await?;
		let dolos_output = self
			.dolos
			.get_transfers(
				main_chain_scripts,
				data_checkpoint,
				max_transfers,
				current_mc_block_hash,
			)
			.await?;
		if reference != dolos_output {
			println!(
				"😱 Dolos source for TokenBridgeDataSource::get_transfers returned different result:\n  dbsync: {reference:?}\n  dolos: {dolos_output:?}"
			)
		}
		Ok(dolos_output)
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
				.await?,
			),
		}),
		block_participation: Arc::new(StakeDistributionDataSourceImplDiff {
			dolos: Arc::new(
				partner_chains_dolos_data_sources::StakeDistributionDataSourceImpl::new(
					dolos_client.clone(),
				),
			),
			dbsync: Arc::new(
				partner_chains_db_sync_data_sources::StakeDistributionDataSourceImpl::new(
					pool.clone(),
					metrics_opt.clone(),
					STAKE_CACHE_SIZE,
				),
			),
		}),
		governed_map: Arc::new(GovernedMapDataSourceImplDiff {
			dolos: Arc::new(partner_chains_dolos_data_sources::GovernedMapDataSourceImpl::new(
				dolos_client.clone(),
			)),
			dbsync: Arc::new(
				partner_chains_db_sync_data_sources::GovernedMapDataSourceImpl::new(
					pool.clone(),
					metrics_opt.clone(),
				)
				.await?,
			),
		}),
		bridge: Arc::new(TokenBridgeDataSourceImplDiff {
			dolos: Arc::new(partner_chains_dolos_data_sources::TokenBridgeDataSourceImpl::new(
				dolos_client.clone(),
			)),
			dbsync: Arc::new(partner_chains_db_sync_data_sources::TokenBridgeDataSourceImpl::new(
				pool,
				metrics_opt,
			)),
		}),
	})
}
