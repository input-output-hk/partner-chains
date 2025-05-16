#![allow(unused_imports)]
#![allow(unused_variables)]
use crate::{PartnerChainsNodeConfig, data_source::PartnerChainsDataSource};
use jsonrpsee::RpcModule;
use pallet_block_producer_fees_rpc::*;
use pallet_block_producer_metadata_rpc::*;
use pallet_session_validator_management_rpc::*;
use pallet_sidechain_rpc::types::GetBestHash;
use pallet_sidechain_rpc::*;
use sidechain_slots::SlotApi;
use sp_api::ProvideRuntimeApi;
use sp_block_producer_fees::BlockProducerFeesApi;
use sp_block_producer_metadata::BlockProducerMetadataApi;
use sp_blockchain::HeaderBackend;
use sp_core::{Decode, Encode};
use sp_runtime::Serialize;
use sp_runtime::traits::Block as BlockT;
use sp_session_validator_management_query::SessionValidatorManagementQuery;
use sp_session_validator_management_query::SessionValidatorManagementQueryApi;
use sp_sidechain::*;
use std::sync::Arc;

pub fn add_sidechain_rpc<T, C, B, CommitteeMember>(
	config: &PartnerChainsNodeConfig,
	data_source: &PartnerChainsDataSource,
	client: Arc<C>,
	module: &mut RpcModule<T>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
	B: BlockT,
	CommitteeMember: Decode + Send + Sync + 'static,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<B>,
	C: GetBestHash<B>,
	C::Api: SlotApi<B>,
	C::Api: GetGenesisUtxo<B>,
	C::Api: GetSidechainStatus<B>,
	SessionValidatorManagementQuery<C, B, CommitteeMember>: SessionValidatorManagementQueryApi,
{
	module.merge(
		SidechainRpc::new(
			client.clone(),
			config.mc_epoch_config.clone(),
			data_source.sidechain_rpc.clone(),
			data_source.time.clone(),
		)
		.into_rpc(),
	)?;

	module.merge(
		SessionValidatorManagementRpc::new(Arc::new(SessionValidatorManagementQuery::new(
			client,
			data_source.authority_selection.clone(),
		)))
		.into_rpc(),
	)?;

	Ok(())
}

#[cfg(feature = "block-participation")]
pub fn add_block_participation_rpc<T, C, B, Metadata, AccountId>(
	config: &PartnerChainsNodeConfig,
	data_source: &PartnerChainsDataSource,
	client: Arc<C>,
	module: &mut RpcModule<T>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
	B: BlockT,
	Metadata: Clone + Decode + Serialize + Send + Sync + 'static,
	AccountId: Clone + Decode + Serialize + Send + Sync + 'static,
	C: Send + Sync + 'static,
	C: ProvideRuntimeApi<B>,
	C: HeaderBackend<B>,
	C::Api: BlockProducerMetadataApi<B, Metadata>,
	C::Api: BlockProducerFeesApi<B, AccountId>,
{
	use sp_blockchain::HeaderBackend;

	module.merge(BlockProducerFeesRpc::new(client.clone()).into_rpc())?;
	module.merge(BlockProducerMetadataRpc::new(client.clone()).into_rpc())?;

	Ok(())
}

#[cfg(not(feature = "block-participation"))]
pub fn add_block_participation_rpc<T, C, B>(
	config: &PartnerChainsNodeConfig,
	data_source: &PartnerChainsDataSource,
	client: Arc<C>,
	module: &mut RpcModule<T>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
	Ok(())
}

#[macro_export]
macro_rules! partner_chains_rpc {
	($config:expr, $data_source:expr, $client:expr, $module:expr) => {{
		$crate::rpc::add_sidechain_rpc($config, $data_source, $client.clone(), &mut $module)?;
		$crate::rpc::add_block_participation_rpc(
			$config,
			$data_source,
			$client.clone(),
			&mut $module,
		)?;
	}};
}
