#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sidechain_domain::{DelegatorKey, MainchainKeyHash, McEpochNumber};
pub use sp_consensus_slots::{Slot, SlotDuration};
use sp_inherents::{InherentIdentifier, IsFatalError};

#[cfg(test)]
mod tests;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"blokpart";

#[derive(Clone, Debug, PartialEq, Eq, Decode, Encode, TypeInfo, PartialOrd, Ord)]
pub struct DelegatorBlockParticipationData<DelegatorId> {
	id: DelegatorId,
	share: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Decode, Encode, TypeInfo, PartialOrd, Ord)]
pub struct BlockProducerParticipationData<BlockProducerId, DelegatorId> {
	block_producer: BlockProducerId,
	block_count: u32,
	delegator_total_shares: u64,
	delegators: Vec<DelegatorBlockParticipationData<DelegatorId>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Decode, Encode, TypeInfo)]
pub struct BlockProductionData<BlockProducerId, DelegatorId> {
	up_to_slot: Slot,
	producer_participation: Vec<BlockProducerParticipationData<BlockProducerId, DelegatorId>>,
}

impl<BlockProducerId, DelegatorId> BlockProductionData<BlockProducerId, DelegatorId> {
	pub fn new(
		up_to_slot: Slot,
		mut producer_participation: Vec<
			BlockProducerParticipationData<BlockProducerId, DelegatorId>,
		>,
	) -> Self
	where
		BlockProducerId: Ord,
		DelegatorId: Ord,
	{
		for breakdown in &mut producer_participation {
			breakdown.delegators.sort()
		}
		producer_participation.sort();
		Self { up_to_slot, producer_participation }
	}

	pub fn up_to_slot(&self) -> Slot {
		self.up_to_slot.clone()
	}

	pub fn producer_participation(
		&self,
	) -> &[BlockProducerParticipationData<BlockProducerId, DelegatorId>] {
		&self.producer_participation
	}
}

#[derive(Encode, PartialEq)]
#[cfg_attr(not(feature = "std"), derive(Debug))]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error, sp_runtime::RuntimeDebug))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Block participation inherent not produced when expected"))]
	InherentRequired,
	#[cfg_attr(feature = "std", error("Block participation inherent produced when not expected"))]
	UnexpectedInherent,
	#[cfg_attr(feature = "std", error("Block participation up_to_slot incorrect"))]
	IncorrectSlotBoundary,
	#[cfg_attr(feature = "std", error("Inherent data provided by the node is invalid"))]
	InvalidInherentData,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

sp_api::decl_runtime_apis! {
	pub trait BlockParticipationApi<BlockProducerId: Decode> {
		/// Should return slot up to which block production data should be released or None.
		fn should_release_data(slot: Slot) -> Option<Slot>;
		fn blocks_produced_up_to_slot(slot: Slot) -> Vec<(Slot, BlockProducerId)>;
		fn target_inherent_id() -> InherentIdentifier;
	}
}

pub trait AsCardanoSPO {
	fn as_cardano_spo(&self) -> Option<MainchainKeyHash>;
}
impl AsCardanoSPO for Option<MainchainKeyHash> {
	fn as_cardano_spo(&self) -> Option<MainchainKeyHash> {
		self.clone()
	}
}

pub trait CardanoDelegator {
	fn from_delegator_key(key: DelegatorKey) -> Self;
}
impl<T: From<DelegatorKey>> CardanoDelegator for T {
	fn from_delegator_key(key: DelegatorKey) -> Self {
		key.into()
	}
}

#[cfg(feature = "std")]
pub mod inherent_data {
	use super::*;
	use alloc::fmt::Debug;
	use core::error::Error;
	use sidechain_domain::mainchain_epoch::*;
	use sidechain_domain::*;
	use sp_api::{ApiError, ApiExt, ProvideRuntimeApi};
	use sp_inherents::{InherentData, InherentDataProvider};
	use sp_runtime::traits::Block as BlockT;
	pub use sp_stake_distribution::StakeDistributionDataSource;
	use std::collections::HashMap;
	use std::hash::Hash;

	#[derive(thiserror::Error, Debug)]
	pub enum InherentDataCreationError<BlockProducerId: Debug> {
		#[error("Runtime API call failed: {0}")]
		ApiError(#[from] ApiError),
		#[error("Data source call failed: {0}")]
		DataSourceError(Box<dyn Error + Send + Sync>),
		#[error("Missing epoch {0} data for {1:?}")]
		DataMissing(McEpochNumber, BlockProducerId),
		#[error("Offset of {1} can not be applied to main chain epoch {0}")]
		McEpochBelowOffset(McEpochNumber, u32),
	}

	/// Inherent data provider for block participation data.
	/// This IDP is active only if the `BlockParticipationApi::should_release_data` function returns `Some`.
	/// This IDP provides two sets of inherent data:
	/// - One is the block production data saved under the inherent ID
	///   indicated by `BlockParticipationApi::target_inherent_id()` function, which is intended for consumption by
	///   a chain-specific handler pallet.
	/// - The other is the slot limit returned by `BlockParticipationApi::should_release_data`. This inherent data
	///   is needed for internal operation of the feature and triggers clearing of already handled data
	///   from the block production log pallet.
	#[derive(Debug, Clone, PartialEq)]
	pub enum BlockParticipationInherentDataProvider<BlockProducerId, DelegatorId> {
		Active {
			target_inherent_id: InherentIdentifier,
			block_production_data: BlockProductionData<BlockProducerId, DelegatorId>,
		},
		Inert,
	}

	impl<BlockProducer, Delegator> BlockParticipationInherentDataProvider<BlockProducer, Delegator>
	where
		BlockProducer: Decode + Clone + Hash + Eq + Ord + Debug,
		Delegator: CardanoDelegator + Ord + Debug,
		BlockProducer: AsCardanoSPO,
	{
		pub async fn new_cardano_stake_based_if_pallet_present<Block: BlockT, T>(
			client: &T,
			data_source: &(dyn StakeDistributionDataSource + Send + Sync),
			parent_hash: <Block as BlockT>::Hash,
			current_slot: Slot,
			mc_epoch_config: &MainchainEpochConfig,
			slot_duration: SlotDuration,
		) -> Result<Self, InherentDataCreationError<BlockProducer>>
		where
			T: ProvideRuntimeApi<Block> + Send + Sync,
			T::Api: BlockParticipationApi<Block, BlockProducer>,
		{
			if client
				.runtime_api()
				.has_api::<dyn BlockParticipationApi<Block, BlockProducer>>(parent_hash)?
			{
				Self::new_cardano_stake_based(
					client,
					data_source,
					parent_hash,
					current_slot,
					mc_epoch_config,
					slot_duration,
				)
				.await
			} else {
				Ok(Self::Inert)
			}
		}
		pub async fn new_cardano_stake_based<Block: BlockT, T>(
			client: &T,
			data_source: &(dyn StakeDistributionDataSource + Send + Sync),
			parent_hash: <Block as BlockT>::Hash,
			current_slot: Slot,
			mc_epoch_config: &MainchainEpochConfig,
			slot_duration: SlotDuration,
		) -> Result<Self, InherentDataCreationError<BlockProducer>>
		where
			T: ProvideRuntimeApi<Block> + Send + Sync,
			T::Api: BlockParticipationApi<Block, BlockProducer>,
		{
			let api = client.runtime_api();
			let Some(up_to_slot) = api.should_release_data(parent_hash, current_slot)? else {
				log::debug!("💤︎ Skipping computing block participation data this block...");
				return Ok(Self::Inert);
			};
			let blocks_produced_up_to_slot =
				api.blocks_produced_up_to_slot(parent_hash, up_to_slot)?;
			let target_inherent_id = api.target_inherent_id(parent_hash)?;

			let block_counts_by_epoch_and_producer = Self::count_blocks_by_epoch_and_producer(
				blocks_produced_up_to_slot,
				mc_epoch_config,
				slot_duration,
			)?;

			let mut production_summaries = vec![];
			for (mc_epoch, producer_blocks) in block_counts_by_epoch_and_producer {
				let stake_distribution =
					Self::fetch_delegations(mc_epoch, producer_blocks.keys().cloned(), data_source)
						.await?;
				for (producer, block_count) in producer_blocks {
					let breakdown = Self::production_breakdown_for(
						mc_epoch,
						producer,
						block_count,
						&stake_distribution,
					)?;

					production_summaries.push(breakdown);
				}
			}

			Ok(Self::Active {
				target_inherent_id,
				block_production_data: BlockProductionData::new(up_to_slot, production_summaries),
			})
		}

		fn production_breakdown_for(
			mc_epoch: McEpochNumber,
			block_producer: BlockProducer,
			block_count: u32,
			distribution: &StakeDistribution,
		) -> Result<
			BlockProducerParticipationData<BlockProducer, Delegator>,
			InherentDataCreationError<BlockProducer>,
		> {
			let (beneficiary_total_share, beneficiaries) = match block_producer.as_cardano_spo() {
				None => (0, vec![]),
				Some(cardano_producer) => {
					let PoolDelegation { total_stake, delegators } =
						distribution.0.get(&cardano_producer).ok_or_else(|| {
							InherentDataCreationError::DataMissing(mc_epoch, block_producer.clone())
						})?;
					let beneficiaries = delegators
						.iter()
						.map(|(delegator_key, stake_amount)| DelegatorBlockParticipationData {
							id: Delegator::from_delegator_key(delegator_key.clone()),
							share: stake_amount.0.into(),
						})
						.collect();
					(total_stake.0, beneficiaries)
				},
			};

			Ok(BlockProducerParticipationData {
				block_producer,
				block_count,
				delegator_total_shares: beneficiary_total_share,
				delegators: beneficiaries,
			})
		}

		async fn fetch_delegations(
			mc_epoch: McEpochNumber,
			producers: impl Iterator<Item = BlockProducer>,
			data_source: &(dyn StakeDistributionDataSource + Send + Sync),
		) -> Result<StakeDistribution, InherentDataCreationError<BlockProducer>> {
			let pools: Vec<_> = producers.flat_map(|p| p.as_cardano_spo()).collect();
			data_source
				.get_stake_pool_delegation_distribution_for_pools(mc_epoch, &pools)
				.await
				.map_err(InherentDataCreationError::DataSourceError)
		}

		// The number of main chain epochs back a Partner Chain queries for candidate stake delegation.
		// This offset is necessary to fetch the same data that was used to select committee members.
		const DATA_MC_EPOCH_OFFSET: u32 = 2;

		fn data_mc_epoch_for_slot(
			slot: Slot,
			slot_duration: SlotDuration,
			mc_epoch_config: &MainchainEpochConfig,
		) -> Result<McEpochNumber, InherentDataCreationError<BlockProducer>> {
			let timestamp = Timestamp::from_unix_millis(
				slot.timestamp(slot_duration)
					.expect("Timestamp for past slots can not overflow")
					.as_millis(),
			);
			let mc_epoch = mc_epoch_config
				.timestamp_to_mainchain_epoch(timestamp)
				.expect("Mainchain epoch for past slots exists");

			(mc_epoch.0.checked_sub(Self::DATA_MC_EPOCH_OFFSET))
				.ok_or(InherentDataCreationError::McEpochBelowOffset(
					mc_epoch,
					Self::DATA_MC_EPOCH_OFFSET,
				))
				.map(McEpochNumber)
		}

		fn count_blocks_by_epoch_and_producer(
			slot_producers: Vec<(Slot, BlockProducer)>,
			mc_epoch_config: &MainchainEpochConfig,
			slot_duration: SlotDuration,
		) -> Result<
			HashMap<McEpochNumber, HashMap<BlockProducer, u32>>,
			InherentDataCreationError<BlockProducer>,
		> {
			let mut epoch_producers: HashMap<McEpochNumber, HashMap<BlockProducer, u32>> =
				HashMap::new();

			for (slot, producer) in slot_producers {
				let mc_epoch = Self::data_mc_epoch_for_slot(slot, slot_duration, mc_epoch_config)?;
				let producer_block_count =
					epoch_producers.entry(mc_epoch).or_default().entry(producer).or_default();

				*producer_block_count += 1;
			}

			Ok(epoch_producers)
		}
	}

	#[async_trait::async_trait]
	impl<BlockProducerId, DelegatorId> InherentDataProvider
		for BlockParticipationInherentDataProvider<BlockProducerId, DelegatorId>
	where
		DelegatorId: Encode + Send + Sync,
		BlockProducerId: Encode + Send + Sync,
	{
		async fn provide_inherent_data(
			&self,
			inherent_data: &mut InherentData,
		) -> Result<(), sp_inherents::Error> {
			if let Self::Active { target_inherent_id, block_production_data } = &self {
				inherent_data.put_data(target_inherent_id.clone(), block_production_data)?;
				inherent_data.put_data(INHERENT_IDENTIFIER, &block_production_data.up_to_slot)?;
			}
			Ok(())
		}

		async fn try_handle_error(
			&self,
			identifier: &InherentIdentifier,
			mut error: &[u8],
		) -> Option<Result<(), sp_inherents::Error>> {
			if *identifier == INHERENT_IDENTIFIER {
				let err = match InherentError::decode(&mut error) {
					Ok(error) => Box::from(error),
					Err(decoding_err) => Box::from(format!(
						"Undecodable block production inherent error: {decoding_err:?}"
					)),
				};

				Some(Err(sp_inherents::Error::Application(err)))
			} else {
				None
			}
		}
	}
}
