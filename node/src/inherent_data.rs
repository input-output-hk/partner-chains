use crate::main_chain_follower::DataSources;
use authority_selection_inherents::ariadne_inherent_data_provider::AriadneInherentDataProvider as AriadneIDP;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use derive_new::new;
use epoch_derivation::EpochConfig;
use jsonrpsee::core::async_trait;
use sc_consensus_aura::{find_pre_digest, SlotDuration};
use sc_service::Arc;
use sidechain_domain::{McBlockHash, ScEpochNumber};
use sidechain_mc_hash::McHashInherentDataProvider as McHashIDP;
use sidechain_runtime::{
	opaque::{Block, SessionKeys},
	BeneficiaryId, CrossChainPublic,
};
use sidechain_slots::ScSlotConfig;
use sp_api::ProvideRuntimeApi;
use sp_block_rewards::BlockBeneficiaryInherentProvider;
use sp_blockchain::HeaderBackend;
use sp_consensus_aura::{
	inherents::InherentDataProvider as AuraIDP, sr25519::AuthorityPair as AuraPair, Slot,
};
use sp_core::Pair;
use sp_inherents::CreateInherentDataProviders;
use sp_native_token_management::{
	NativeTokenManagementApi, NativeTokenManagementInherentDataProvider as NativeTokenIDP,
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
	data_sources: DataSources,
}

#[async_trait]
impl<T> CreateInherentDataProviders<Block, ()> for ProposalCIDP<T>
where
	T: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	T: HeaderBackend<Block>,
	T::Api: SessionValidatorManagementApi<
		Block,
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	T::Api: NativeTokenManagementApi<Block>,
{
	type InherentDataProviders = (
		AuraIDP,
		TimestampIDP,
		McHashIDP,
		AriadneIDP,
		BlockBeneficiaryInherentProvider<BeneficiaryId>,
		NativeTokenIDP,
	);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		_extra_args: (),
	) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
		let Self { config, client, data_sources } = self;
		let CreateInherentDataConfig { epoch_config, sc_slot_config, time_source } = config;

		let (slot, timestamp) =
			timestamp_and_slot_cidp(sc_slot_config.slot_duration, time_source.clone());
		let mc_hash = McHashIDP::new_proposal(
			data_sources.block.as_ref(),
			*slot,
			sc_slot_config.slot_duration,
		)
		.await?;

		let ariadne_data_provider = AriadneIDP::new(
			client.as_ref(),
			sc_slot_config,
			epoch_config,
			parent_hash,
			*slot,
			data_sources.candidate.as_ref(),
			mc_hash.mc_epoch(),
		)
		.await?;
		let block_beneficiary_provider =
			BlockBeneficiaryInherentProvider::<BeneficiaryId>::from_env(
				"SIDECHAIN_BLOCK_BENEFICIARY",
			)?;

		let native_token = NativeTokenIDP::new(
			client.clone(),
			data_sources.native_token.as_ref(),
			mc_hash.mc_hash(),
			parent_hash.clone(),
		)
		.await?;

		Ok((
			slot,
			timestamp,
			mc_hash,
			ariadne_data_provider,
			block_beneficiary_provider,
			native_token,
		))
	}
}

#[derive(new)]
pub struct VerifierCIDP<T> {
	config: CreateInherentDataConfig,
	client: Arc<T>,
	data_sources: DataSources,
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
		SessionKeys,
		CrossChainPublic,
		AuthoritySelectionInputs,
		ScEpochNumber,
	>,
	T::Api: NativeTokenManagementApi<Block>,
{
	type InherentDataProviders = (TimestampIDP, AriadneIDP, NativeTokenIDP);

	async fn create_inherent_data_providers(
		&self,
		parent_hash: <Block as BlockT>::Hash,
		(verified_block_slot, mc_hash): (Slot, McBlockHash),
	) -> Result<Self::InherentDataProviders, Box<dyn Error + Send + Sync>> {
		let Self { config, client, data_sources } = self;
		let CreateInherentDataConfig { epoch_config, sc_slot_config, time_source, .. } = config;

		let timestamp = TimestampIDP::new(Timestamp::new(time_source.get_current_time_millis()));
		let parent_header = client.expect_header(parent_hash)?;
		let parent_slot = slot_from_predigest(&parent_header)?;
		let mc_state_reference = McHashIDP::new_verification(
			parent_header,
			parent_slot,
			verified_block_slot,
			mc_hash.clone(),
			config.slot_duration(),
			data_sources.block.as_ref(),
		)
		.await?;

		let ariadne_data_provider = AriadneIDP::new(
			client.as_ref(),
			sc_slot_config,
			epoch_config,
			parent_hash,
			verified_block_slot,
			data_sources.candidate.as_ref(),
			mc_state_reference.epoch,
		)
		.await?;

		let native_token = NativeTokenIDP::new(
			client.clone(),
			data_sources.native_token.as_ref(),
			mc_hash,
			parent_hash.clone(),
		)
		.await?;

		Ok((timestamp, ariadne_data_provider, native_token))
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
	pub epoch_config: EpochConfig,
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
