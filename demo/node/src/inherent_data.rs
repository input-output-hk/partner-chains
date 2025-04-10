use authority_selection_inherents::ariadne_inherent_data_provider::AriadneInherentDataProvider as AriadneIDP;
use authority_selection_inherents::authority_selection_inputs::{
	AuthoritySelectionDataSource, AuthoritySelectionInputs,
};
use authority_selection_inherents::CommitteeMember;
use derive_new::new;
use jsonrpsee::core::async_trait;
use partner_chains_demo_runtime::{
	opaque::{Block, SessionKeys},
	BlockAuthor, CrossChainPublic,
};
use sc_consensus_aura::{find_pre_digest, SlotDuration};
use sc_service::Arc;
use sidechain_domain::{
	mainchain_epoch::MainchainEpochConfig, DelegatorKey, McBlockHash, ScEpochNumber,
};
use sidechain_mc_hash::{McHashDataSource, McHashInherentDataProvider as McHashIDP};
use sidechain_slots::ScSlotConfig;
use sp_api::ProvideRuntimeApi;
use sp_block_participation::{
	inherent_data::{BlockParticipationDataSource, BlockParticipationInherentDataProvider},
	BlockParticipationApi,
};
use sp_block_production_log::{BlockAuthorInherentProvider, BlockProductionLogApi};
use sp_blockchain::HeaderBackend;
use sp_consensus_aura::{
	inherents::InherentDataProvider as AuraIDP, sr25519::AuthorityPair as AuraPair, Slot,
};
use sp_core::Pair;
use sp_inherents::CreateInherentDataProviders;
use sp_native_token_management::{
	NativeTokenManagementApi, NativeTokenManagementDataSource,
	NativeTokenManagementInherentDataProvider as NativeTokenIDP,
};
use sp_partner_chains_consensus_aura::CurrentSlotProvider;
use sp_runtime::traits::{Block as BlockT, Header, Zero};
use sp_session_validator_management::SessionValidatorManagementApi;
use sp_timestamp::{InherentDataProvider as TimestampIDP, Timestamp};
use std::error::Error;
use time_source::TimeSource;

#[derive(new)]
pub struct ProposalCIDP<T> {
	config: CreateInherentDataConfig,
	client: Arc<T>,
	mc_hash_data_source: Arc<dyn McHashDataSource + Send + Sync>,
	authority_selection_data_source: Arc<dyn AuthoritySelectionDataSource + Send + Sync>,
	native_token_data_source: Arc<dyn NativeTokenManagementDataSource + Send + Sync>,
	block_participation_data_source: Arc<dyn BlockParticipationDataSource + Send + Sync>,
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
{
	type InherentDataProviders = (
		AuraIDP,
		TimestampIDP,
		McHashIDP,
		AriadneIDP,
		BlockAuthorInherentProvider<BlockAuthor>,
		NativeTokenIDP,
		BlockParticipationInherentDataProvider<BlockAuthor, DelegatorKey>,
	);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		_extra_args: (),
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		let Self {
			config,
			client,
			mc_hash_data_source,
			authority_selection_data_source,
			native_token_data_source,
			block_participation_data_source,
		} = self;
		let CreateInherentDataConfig { mc_epoch_config, sc_slot_config, time_source } = config;

		let (slot, timestamp) =
			timestamp_and_slot_cidp(sc_slot_config.slot_duration, time_source.clone());
		let parent_header = client.expect_header(parent_hash)?;
		let mc_hash = McHashIDP::new_proposal(
			parent_header,
			mc_hash_data_source.as_ref(),
			*slot,
			sc_slot_config.slot_duration,
		)
		.await?;

		let ariadne_data_provider = AriadneIDP::new(
			client.as_ref(),
			sc_slot_config,
			mc_epoch_config,
			parent_hash,
			*slot,
			authority_selection_data_source.as_ref(),
			mc_hash.mc_epoch(),
		)
		.await?;
		let block_producer_id_provider =
			BlockAuthorInherentProvider::new(client.as_ref(), parent_hash, *slot)?;

		let native_token = NativeTokenIDP::new(
			client.clone(),
			native_token_data_source.as_ref(),
			mc_hash.mc_hash(),
			parent_hash,
		)
		.await?;

		let payouts = BlockParticipationInherentDataProvider::new(
			client.as_ref(),
			block_participation_data_source.as_ref(),
			parent_hash,
			*slot,
			mc_epoch_config,
			config.sc_slot_config.slot_duration,
		)
		.await?;

		Ok((
			slot,
			timestamp,
			mc_hash,
			ariadne_data_provider,
			block_producer_id_provider,
			native_token,
			payouts,
		))
	}
}

#[derive(new)]
pub struct VerifierCIDP<T> {
	config: CreateInherentDataConfig,
	client: Arc<T>,
	mc_hash_data_source: Arc<dyn McHashDataSource + Send + Sync>,
	authority_selection_data_source: Arc<dyn AuthoritySelectionDataSource + Send + Sync>,
	native_token_data_source: Arc<dyn NativeTokenManagementDataSource + Send + Sync>,
	block_participation_data_source: Arc<dyn BlockParticipationDataSource + Send + Sync>,
}

impl<T: Send + Sync> CurrentSlotProvider for VerifierCIDP<T> {
	fn slot(&self) -> Slot {
		*timestamp_and_slot_cidp(self.config.slot_duration(), self.config.time_source.clone()).0
	}
}

#[async_trait]
impl<T> CreateInherentDataProviders<Block, (Slot, McBlockHash)> for VerifierCIDP<T>
where
	T: ProvideRuntimeApi<Block> + Send + Sync + HeaderBackend<Block>,
	T::Api: SessionValidatorManagementApi<
		Block,
		CommitteeMember<CrossChainPublic, SessionKeys>,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	T::Api: NativeTokenManagementApi<Block>,
	T::Api: BlockProductionLogApi<Block, CommitteeMember<CrossChainPublic, SessionKeys>>,
	T::Api: BlockParticipationApi<Block, BlockAuthor>,
{
	type InherentDataProviders = (
		TimestampIDP,
		AriadneIDP,
		BlockAuthorInherentProvider<BlockAuthor>,
		NativeTokenIDP,
		BlockParticipationInherentDataProvider<BlockAuthor, DelegatorKey>,
	);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		(verified_block_slot, mc_hash): (Slot, McBlockHash),
	) -> Result<Self::InherentDataProviders, Box<dyn Error + Send + Sync>> {
		let Self {
			config,
			client,
			mc_hash_data_source,
			authority_selection_data_source,
			native_token_data_source,
			block_participation_data_source,
		} = self;
		let CreateInherentDataConfig { mc_epoch_config, sc_slot_config, time_source, .. } = config;

		let timestamp = TimestampIDP::new(Timestamp::new(time_source.get_current_time_millis()));
		let parent_header = client.expect_header(parent_hash)?;
		let parent_slot = slot_from_predigest(&parent_header)?;
		let mc_state_reference = McHashIDP::new_verification(
			parent_header,
			parent_slot,
			verified_block_slot,
			mc_hash.clone(),
			config.slot_duration(),
			mc_hash_data_source.as_ref(),
		)
		.await?;

		let ariadne_data_provider = AriadneIDP::new(
			client.as_ref(),
			sc_slot_config,
			mc_epoch_config,
			parent_hash,
			verified_block_slot,
			authority_selection_data_source.as_ref(),
			mc_state_reference.epoch,
		)
		.await?;

		let native_token = NativeTokenIDP::new(
			client.clone(),
			native_token_data_source.as_ref(),
			mc_hash,
			parent_hash,
		)
		.await?;

		let block_producer_id_provider =
			BlockAuthorInherentProvider::new(client.as_ref(), parent_hash, verified_block_slot)?;

		let payouts = BlockParticipationInherentDataProvider::new(
			client.as_ref(),
			block_participation_data_source.as_ref(),
			parent_hash,
			verified_block_slot,
			mc_epoch_config,
			config.sc_slot_config.slot_duration,
		)
		.await?;

		Ok((timestamp, ariadne_data_provider, block_producer_id_provider, native_token, payouts))
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

#[derive(new, Clone)]
pub(crate) struct CreateInherentDataConfig {
	pub mc_epoch_config: MainchainEpochConfig,
	// TODO ETCM-4079 make sure that this struct can be instantiated only if sidechain epoch duration is divisible by slot_duration
	pub sc_slot_config: ScSlotConfig,
	pub time_source: Arc<dyn TimeSource + Send + Sync>,
}

impl CreateInherentDataConfig {
	pub fn slot_duration(&self) -> SlotDuration {
		self.sc_slot_config.slot_duration
	}
}

fn timestamp_and_slot_cidp(
	slot_duration: SlotDuration,
	time_source: Arc<dyn TimeSource + Send + Sync>,
) -> (AuraIDP, TimestampIDP) {
	let timestamp = TimestampIDP::new(Timestamp::new(time_source.get_current_time_millis()));
	let slot = AuraIDP::from_timestamp_and_slot_duration(*timestamp, slot_duration);
	(slot, timestamp)
}
