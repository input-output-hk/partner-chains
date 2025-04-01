//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use crate::main_chain_follower::DataSources;
use authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi;
use authority_selection_inherents::{
	authority_selection_inputs::AuthoritySelectionInputs, CommitteeMember,
};
use jsonrpsee::RpcModule;
use pallet_block_producer_metadata_rpc::*;
use pallet_session_validator_management_rpc::*;
use pallet_sidechain_rpc::*;
use partner_chains_demo_runtime::{
	opaque::{Block, SessionKeys},
	AccountId, Balance, Nonce,
};
use partner_chains_demo_runtime::{BlockNumber, BlockProducerMetadataType, CrossChainPublic, Hash};
use sc_consensus_grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool_api::TransactionPool;
use sidechain_domain::mainchain_epoch::MainchainEpochConfig;
use sidechain_domain::ScEpochNumber;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_session_validator_management_query::SessionValidatorManagementQuery;
use std::sync::Arc;
use time_source::TimeSource;

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, B, T> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// Main chain follower data sources.
	pub main_chain_follower_data_sources: DataSources,
	/// Source of system time
	pub time_source: Arc<T>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, B, T>(
	deps: FullDeps<C, P, B, T>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>,
	C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
	C: Send + Sync + 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BlockBuilder<Block>,
	C::Api: sp_consensus_aura::AuraApi<Block, sp_consensus_aura::sr25519::AuthorityId>,
	C::Api: sidechain_slots::SlotApi<Block>,
	C::Api: sp_sidechain::GetGenesisUtxo<Block>,
	C::Api: sp_sidechain::GetSidechainStatus<Block>,
	C::Api: sp_block_producer_metadata::BlockProducerMetadataApi<Block, BlockProducerMetadataType>,
	C::Api: sp_session_validator_management::SessionValidatorManagementApi<
		Block,
		CommitteeMember<CrossChainPublic, SessionKeys>,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	C::Api: CandidateValidationApi<Block>,
	P: TransactionPool + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashingFor<Block>>,
	T: TimeSource + Send + Sync + 'static,
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut module = RpcModule::new(());
	let FullDeps { client, pool, grandpa, main_chain_follower_data_sources, time_source } = deps;

	module.merge(System::new(client.clone(), pool.clone()).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	module.merge(
		SidechainRpc::new(
			client.clone(),
			MainchainEpochConfig::read_from_env().unwrap(),
			main_chain_follower_data_sources.sidechain_rpc.clone(),
			time_source.clone(),
		)
		.into_rpc(),
	)?;
	module.merge(BlockProducerMetadataRpc::new(client.clone()).into_rpc())?;

	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;
	module.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;
	module.merge(
		SessionValidatorManagementRpc::new(Arc::new(SessionValidatorManagementQuery::new(
			client.clone(),
			main_chain_follower_data_sources.authority_selection.clone(),
		)))
		.into_rpc(),
	)?;

	// Extend this RPC with a custom API by using the following syntax.
	// `YourRpcStruct` should have a reference to a client, which is needed
	// to call into the runtime.
	// `module.merge(YourRpcTrait::into_rpc(YourRpcStruct::new(ReferenceToClient, ...)))?;`

	Ok(module)
}
