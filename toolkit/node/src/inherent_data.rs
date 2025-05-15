use crate::data_source::PartnerChainsDataSource;
use authority_selection_inherents::ariadne_inherent_data_provider::AriadneInherentDataProvider as AriadneIDP;
use authority_selection_inherents::authority_selection_inputs::AuthoritySelectionInputs;
use sidechain_domain::*;
use sidechain_mc_hash::McHashInherentDataProvider as McHashIDP;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{Decode, Encode};
use sp_runtime::traits::Block as BlockT;
use sp_session_validator_management::CommitteeMember as CommitteeMemberT;
use sp_session_validator_management::SessionValidatorManagementApi;
use std::{fmt::Debug, hash::Hash, marker::PhantomData, sync::Arc};

#[cfg(feature = "governed-map")]
use sp_governed_map::{GovernedMapIDPApi, GovernedMapInherentDataProvider};

#[cfg(feature = "native-token-management")]
use sp_native_token_management::{
	NativeTokenManagementApi, NativeTokenManagementInherentDataProvider as NativeTokenIDP,
};

#[cfg(feature = "block-participation")]
use {
	sp_block_participation::{
		AsCardanoSPO, BlockParticipationApi, Slot,
		inherent_data::BlockParticipationInherentDataProvider,
	},
	sp_block_production_log::{BlockAuthorInherentProvider, BlockProductionLogApi},
};

pub trait PartnerChainsInherentDataTypes {
	type CommitteeMember: CommitteeMemberT + Decode + Encode + Send + Sync;
	#[cfg(feature = "block-participation")]
	type DelegatorKey: Debug + Ord + From<sidechain_domain::DelegatorKey> + Encode + Send + Sync;
	#[cfg(feature = "block-participation")]
	type BlockAuthor: Clone
		+ Debug
		+ Encode
		+ Decode
		+ From<Self::CommitteeMember>
		+ AsCardanoSPO
		+ Ord
		+ Hash
		+ Send
		+ Sync
		+ 'static;
}

pub struct PartnerChainsInherentDataProvider<T: PartnerChainsInherentDataTypes> {
	pub mc_hash: McHashIDP,
	pub ariadne: AriadneIDP,
	#[cfg(feature = "block-participation")]
	pub block_author: BlockAuthorInherentProvider<T::BlockAuthor>,
	#[cfg(feature = "native-token-management")]
	pub native_token: NativeTokenIDP,
	#[cfg(feature = "block-participation")]
	pub block_participation:
		BlockParticipationInherentDataProvider<T::BlockAuthor, T::DelegatorKey>,
	#[cfg(feature = "governed-map")]
	pub governed_map: GovernedMapInherentDataProvider,
	_phantom: PhantomData<T>,
}

impl<T: PartnerChainsInherentDataTypes> PartnerChainsInherentDataProvider<T> {
	pub async fn new_proposal<Block: BlockT, Client>(
		slot: Slot,
		parent_hash: Block::Hash,
		config: crate::PartnerChainsNodeConfig,
		data_source: PartnerChainsDataSource,
		client: Arc<Client>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	where
		Client: ProvideRuntimeApi<Block> + Send + Sync + 'static,
		Client: HeaderBackend<Block>,
		<T::CommitteeMember as CommitteeMemberT>::AuthorityKeys: Decode + Encode,
		<T::CommitteeMember as CommitteeMemberT>::AuthorityId: Decode + Encode,
		Client::Api: SessionValidatorManagementApi<
				Block,
				T::CommitteeMember,
				AuthoritySelectionInputs,
				ScEpochNumber,
			>,
		Client::Api: BlockProductionLogApi<Block, T::CommitteeMember>,
		Client::Api: NativeTokenManagementApi<Block>,
		Client::Api: BlockParticipationApi<Block, T::BlockAuthor>,
		Client::Api: GovernedMapIDPApi<Block>,
		T::DelegatorKey: Debug + Ord + From<sidechain_domain::DelegatorKey>,
	{
		let parent_header = client.expect_header(parent_hash)?;

		let mc_hash_idp = McHashIDP::new_proposal(
			parent_header,
			data_source.mc_hash.as_ref(),
			slot,
			config.sc_slot_config.slot_duration,
		)
		.await?;

		let ariadne_idp = AriadneIDP::new(
			client.as_ref(),
			&config.sc_slot_config,
			&config.mc_epoch_config,
			parent_hash,
			slot,
			data_source.authority_selection.as_ref(),
			mc_hash_idp.mc_epoch(),
		)
		.await?;

		#[cfg(feature = "block-participation")]
		let block_producer_idp: BlockAuthorInherentProvider<T::BlockAuthor> =
			BlockAuthorInherentProvider::new::<Client, T::CommitteeMember, Block>(
				client.as_ref(),
				parent_hash,
				slot,
			)?;

		#[cfg(feature = "native-token-management")]
		let native_token_idp = NativeTokenIDP::new(
			client.clone(),
			data_source.native_token.as_ref(),
			mc_hash_idp.mc_hash(),
			mc_hash_idp.previous_mc_hash(),
			parent_hash,
		)
		.await?;

		#[cfg(feature = "block-participation")]
		let block_participation_idp: BlockParticipationInherentDataProvider<
			T::BlockAuthor,
			T::DelegatorKey,
		> = BlockParticipationInherentDataProvider::new(
			client.as_ref(),
			data_source.block_participation.as_ref(),
			parent_hash,
			slot,
			&config.mc_epoch_config,
			config.sc_slot_config.slot_duration,
		)
		.await?;

		#[cfg(feature = "governed-map")]
		let governed_map_idp = GovernedMapInherentDataProvider::new(
			client.as_ref(),
			parent_hash,
			mc_hash_idp.mc_hash(),
			mc_hash_idp.previous_mc_hash(),
			data_source.governed_map.as_ref(),
		)
		.await?;

		Ok(Self {
			mc_hash: mc_hash_idp,
			ariadne: ariadne_idp,
			#[cfg(feature = "native-token-management")]
			native_token: native_token_idp,
			#[cfg(feature = "block-participation")]
			block_author: block_producer_idp,
			#[cfg(feature = "block-participation")]
			block_participation: block_participation_idp,
			#[cfg(feature = "governed-map")]
			governed_map: governed_map_idp,
			_phantom: PhantomData,
		})
	}

	pub async fn new_verification<Block: BlockT, Client>(
		verified_block_slot: Slot,
		verified_mc_hash: McBlockHash,
		parent_slot: Option<Slot>,
		parent_hash: Block::Hash,
		config: crate::PartnerChainsNodeConfig,
		data_source: PartnerChainsDataSource,
		client: Arc<Client>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	where
		Client: ProvideRuntimeApi<Block> + Send + Sync + 'static,
		Client: HeaderBackend<Block>,
		<T::CommitteeMember as CommitteeMemberT>::AuthorityKeys: Decode + Encode,
		<T::CommitteeMember as CommitteeMemberT>::AuthorityId: Decode + Encode,
		Client::Api: SessionValidatorManagementApi<
				Block,
				T::CommitteeMember,
				AuthoritySelectionInputs,
				ScEpochNumber,
			>,
		Client::Api: BlockProductionLogApi<Block, T::CommitteeMember>,
		Client::Api: NativeTokenManagementApi<Block>,
		Client::Api: BlockParticipationApi<Block, T::BlockAuthor>,
		Client::Api: GovernedMapIDPApi<Block>,
		T::DelegatorKey: Debug + Ord + From<sidechain_domain::DelegatorKey>,
	{
		let parent_header = client.expect_header(parent_hash)?;
		let mc_hash_idp = McHashIDP::new_verification(
			parent_header,
			parent_slot,
			verified_block_slot,
			verified_mc_hash.clone(),
			config.sc_slot_config.slot_duration.clone(),
			data_source.mc_hash.as_ref(),
		)
		.await?;

		let ariadne_idp = AriadneIDP::new(
			client.as_ref(),
			&config.sc_slot_config,
			&config.mc_epoch_config,
			parent_hash,
			verified_block_slot,
			data_source.authority_selection.as_ref(),
			mc_hash_idp.epoch,
		)
		.await?;

		#[cfg(feature = "native-token-management")]
		let native_token_idp = NativeTokenIDP::new(
			client.clone(),
			data_source.native_token.as_ref(),
			verified_mc_hash.clone(),
			mc_hash_idp.previous_mc_hash(),
			parent_hash,
		)
		.await?;

		#[cfg(feature = "block-participation")]
		let block_producer_idp: BlockAuthorInherentProvider<T::BlockAuthor> =
			BlockAuthorInherentProvider::new::<Client, T::CommitteeMember, Block>(
				client.as_ref(),
				parent_hash,
				verified_block_slot,
			)?;

		#[cfg(feature = "block-participation")]
		let block_participation_idp: BlockParticipationInherentDataProvider<
			T::BlockAuthor,
			T::DelegatorKey,
		> = BlockParticipationInherentDataProvider::new(
			client.as_ref(),
			data_source.block_participation.as_ref(),
			parent_hash,
			verified_block_slot,
			&config.mc_epoch_config,
			config.sc_slot_config.slot_duration,
		)
		.await?;

		#[cfg(feature = "governed-map")]
		let governed_map_idp = GovernedMapInherentDataProvider::new(
			client.as_ref(),
			parent_hash,
			verified_mc_hash,
			mc_hash_idp.previous_mc_hash(),
			data_source.governed_map.as_ref(),
		)
		.await?;

		Ok(Self {
			mc_hash: mc_hash_idp,
			ariadne: ariadne_idp,
			#[cfg(feature = "native-token-management")]
			native_token: native_token_idp,
			#[cfg(feature = "block-participation")]
			block_author: block_producer_idp,
			#[cfg(feature = "block-participation")]
			block_participation: block_participation_idp,
			#[cfg(feature = "governed-map")]
			governed_map: governed_map_idp,
			_phantom: PhantomData,
		})
	}
}

#[async_trait::async_trait]
impl<T: PartnerChainsInherentDataTypes + Send + Sync> sp_inherents::InherentDataProvider
	for PartnerChainsInherentDataProvider<T>
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut sp_inherents::InherentData,
	) -> Result<(), sp_inherents::Error> {
		self.mc_hash.provide_inherent_data(inherent_data).await?;
		self.ariadne.provide_inherent_data(inherent_data).await?;

		#[cfg(feature = "block-participation")]
		self.block_author.provide_inherent_data(inherent_data).await?;

		#[cfg(feature = "native-token-management")]
		self.native_token.provide_inherent_data(inherent_data).await?;

		#[cfg(feature = "block-participation")]
		self.block_participation.provide_inherent_data(inherent_data).await?;

		#[cfg(feature = "governed-map")]
		self.governed_map.provide_inherent_data(inherent_data).await?;

		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &sp_inherents::InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if let Some(Err(err)) = self.mc_hash.try_handle_error(identifier, error).await {
			return Some(Err(err));
		}
		if let Some(Err(err)) = self.ariadne.try_handle_error(identifier, error).await {
			return Some(Err(err));
		}
		#[cfg(feature = "block-participation")]
		if let Some(Err(err)) = self.block_author.try_handle_error(identifier, error).await {
			return Some(Err(err));
		}
		#[cfg(feature = "native-token-management")]
		if let Some(Err(err)) = self.native_token.try_handle_error(identifier, error).await {
			return Some(Err(err));
		}
		#[cfg(feature = "block-participation")]
		if let Some(Err(err)) = self.block_participation.try_handle_error(identifier, error).await {
			return Some(Err(err));
		}
		#[cfg(feature = "governed-map")]
		if let Some(Err(err)) = self.governed_map.try_handle_error(identifier, error).await {
			return Some(Err(err));
		}
		None
	}
}
