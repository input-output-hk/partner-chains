//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use authority_selection_inherents::filter_invalid_candidates::CandidateValidationApi;
use authority_selection_inherents::{
	CommitteeMember, authority_selection_inputs::AuthoritySelectionInputs,
};
use jsonrpsee::RpcModule;
use partner_chains_demo_runtime::{
	AccountId, Balance, Nonce,
	opaque::{Block, SessionKeys},
};
use partner_chains_demo_runtime::{BlockNumber, BlockProducerMetadataType, CrossChainPublic, Hash};
use partner_chains_node::data_source::PartnerChainsDataSource;
use partner_chains_node::{PartnerChainsNodeConfig, partner_chains_rpc};
use sc_consensus_grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
use sc_rpc::SubscriptionTaskExecutor;
use sc_transaction_pool_api::TransactionPool;
use sidechain_domain::ScEpochNumber;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use std::sync::Arc;

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
pub struct FullDeps<C, P, B> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// Data sources.
	pub data_sources: PartnerChainsDataSource,
	/// Partner Chain-specific node configuration
	pub partner_chain_node_config: PartnerChainsNodeConfig,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, B>(
	deps: FullDeps<C, P, B>,
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
	C::Api: sp_block_producer_fees::BlockProducerFeesApi<Block, AccountId>,
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
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};

	let mut module = RpcModule::new(());
	let FullDeps { client, pool, grandpa, partner_chain_node_config, data_sources } = deps;

	module.merge(System::new(client.clone(), pool.clone()).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

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

	partner_chains_rpc!(&partner_chain_node_config, &data_sources, client, module);

	Ok(module)
}
