use authority_selection_inherents::CommitteeMember;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use derive_new::new;
use jsonrpsee::core::async_trait;
use partner_chains_demo_runtime::{
	BlockAuthor, CrossChainPublic,
	opaque::{Block, SessionKeys},
};
use partner_chains_node::{
	PartnerChainsNodeConfig, data_source::PartnerChainsDataSource,
	inherent_data::PartnerChainsInherentDataProvider,
	inherent_data::PartnerChainsInherentDataTypes,
};
use sc_consensus_aura::{SlotDuration, find_pre_digest};
use sc_service::Arc;
use sidechain_domain::{DelegatorKey, McBlockHash, ScEpochNumber};
use sp_api::ProvideRuntimeApi;
use sp_block_participation::BlockParticipationApi;
use sp_block_production_log::BlockProductionLogApi;
use sp_blockchain::HeaderBackend;
use sp_consensus_aura::{
	Slot, inherents::InherentDataProvider as AuraIDP, sr25519::AuthorityPair as AuraPair,
};
use sp_core::Pair;
use sp_governed_map::GovernedMapIDPApi;
use sp_inherents::CreateInherentDataProviders;
use sp_native_token_management::NativeTokenManagementApi;
use sp_partner_chains_consensus_aura::CurrentSlotProvider;
use sp_runtime::traits::{Block as BlockT, Header, Zero};
use sp_session_validator_management::SessionValidatorManagementApi;
use sp_timestamp::{InherentDataProvider as TimestampIDP, Timestamp};
use std::error::Error;
use time_source::TimeSource;

pub struct PartnerChainsTypes;
impl PartnerChainsInherentDataTypes for PartnerChainsTypes {
	type CommitteeMember = CommitteeMember<CrossChainPublic, SessionKeys>;
	type DelegatorKey = DelegatorKey;
	type BlockAuthor = BlockAuthor;
}

#[derive(new)]
pub struct ProposalCIDP<T> {
	config: PartnerChainsNodeConfig,
	client: Arc<T>,
	partner_chain_data_source: PartnerChainsDataSource,
}

#[async_trait]
impl<T> CreateInherentDataProviders<Block, ()> for ProposalCIDP<T>
where
	T: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	T: HeaderBackend<Block>,
	T::Api: SessionValidatorManagementApi<
			Block,
			CommitteeMember<CrossChainPublic, SessionKeys>,
			AuthoritySelectionInputs,
			ScEpochNumber,
		>,
	T::Api: NativeTokenManagementApi<Block>,
	T::Api: BlockProductionLogApi<Block, CommitteeMember<CrossChainPublic, SessionKeys>>,
	T::Api: BlockParticipationApi<Block, BlockAuthor>,
	T::Api: GovernedMapIDPApi<Block>,
{
	type InherentDataProviders =
		(AuraIDP, TimestampIDP, PartnerChainsInherentDataProvider<PartnerChainsTypes>);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		_extra_args: (),
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		let Self { config, client, partner_chain_data_source } = self;
		let PartnerChainsNodeConfig { sc_slot_config, time_source, .. } = config;

		let (slot, timestamp) =
			timestamp_and_slot_cidp(sc_slot_config.slot_duration, time_source.as_ref());

		let partner_chains_idp = PartnerChainsInherentDataProvider::new_proposal(
			*slot,
			parent_hash,
			config.clone(),
			partner_chain_data_source.clone(),
			client.clone(),
		)
		.await?;

		Ok((slot, timestamp, partner_chains_idp))
	}
}

#[derive(new)]
pub struct VerifierCIDP<T> {
	config: PartnerChainsNodeConfig,
	client: Arc<T>,
	partner_chain_data_source: PartnerChainsDataSource,
}

impl<T: Send + Sync> CurrentSlotProvider for VerifierCIDP<T> {
	fn slot(&self) -> Slot {
		*timestamp_and_slot_cidp(
			self.config.sc_slot_config.slot_duration,
			self.config.time_source.as_ref(),
		)
		.0
	}
}

#[async_trait]
impl<T> CreateInherentDataProviders<Block, (Slot, McBlockHash)> for VerifierCIDP<T>
where
	T: ProvideRuntimeApi<Block> + Send + Sync + HeaderBackend<Block> + 'static,
	T::Api: SessionValidatorManagementApi<
			Block,
			CommitteeMember<CrossChainPublic, SessionKeys>,
			AuthoritySelectionInputs,
			ScEpochNumber,
		>,
	T::Api: NativeTokenManagementApi<Block>,
	T::Api: BlockProductionLogApi<Block, CommitteeMember<CrossChainPublic, SessionKeys>>,
	T::Api: BlockParticipationApi<Block, BlockAuthor>,
	T::Api: GovernedMapIDPApi<Block>,
{
	type InherentDataProviders =
		(TimestampIDP, PartnerChainsInherentDataProvider<PartnerChainsTypes>);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		(verified_block_slot, mc_hash): (Slot, McBlockHash),
	) -> Result<Self::InherentDataProviders, Box<dyn Error + Send + Sync>> {
		let Self { config, client, partner_chain_data_source } = self;

		let timestamp =
			TimestampIDP::new(Timestamp::new(config.time_source.get_current_time_millis()));
		let parent_header = client.expect_header(parent_hash)?;
		let parent_slot = slot_from_predigest(&parent_header)?;

		let partner_chains_idp = PartnerChainsInherentDataProvider::new_verification(
			verified_block_slot,
			mc_hash,
			parent_slot,
			parent_hash,
			config.clone(),
			partner_chain_data_source.clone(),
			client.clone(),
		)
		.await?;

		Ok((timestamp, partner_chains_idp))
	}
}

pub fn slot_from_predigest(
	header: &<Block as BlockT>::Header,
) -> Result<Option<Slot>, Box<dyn Error + Send + Sync>> {
	if header.number().is_zero() {
		// genesis block doesn't have a slot
		Ok(None)
	} else {
		Ok(Some(find_pre_digest::<Block, <AuraPair as Pair>::Signature>(header)?))
	}
}

fn timestamp_and_slot_cidp(
	slot_duration: SlotDuration,
	time_source: &(dyn TimeSource + Send + Sync),
) -> (AuraIDP, TimestampIDP) {
	let timestamp = TimestampIDP::new(Timestamp::new(time_source.get_current_time_millis()));
	let slot = AuraIDP::from_timestamp_and_slot_duration(*timestamp, slot_duration);
	(slot, timestamp)
}
