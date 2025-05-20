//! Crate providing block participation data through the inherent data mechanism.
//!
//! ## Purpose of this crate
//!
//! This crate provides logic to compute and expose as inherent data aggregated information on block producers'
//! and their delegators' participation in block production. This feature is implemented in an unusual way in
//! that it publishes inherent data but leaves it to the specific Partner Chain's builders to implement the pallet
//! that will consume this data. This is done because Partner Chains SDK can not anticipate how this data will
//! have to be handled within every Partner Chain. The assumed use of the data provided is paying out of block
//! production reward, however both ledger structure and reward calculation rules are inherently specific to
//! their Partner Chain.
//!
//! ## Outline of operation
//!
//! 1. The inherent data provider calls runtime API to check whether it should release block participation inherent
//!    data (all points below assume this check is positive) and gets the upper slot limit.
//! 2. The inherent data provider retrieves data on block production up to the slot limit using runtime API and Cardano
//!    delegation data using observability data source. The IDP joins and aggregates this data together producing
//!    block participation data.
//! 3. The IDP puts the block participation data into the inherent data of the current block, under the inherent
//!    identifier indicated by the runtime API. This inherent identifier belongs to the Partner Chain's custom
//!    handler crate.
//! 4. The IDP produces an additional "operational" inherent data to signal its own pallet that participation data
//!    has been released.
//! 5. The Partner Chain's custom handler pallet consumes the block participation inherent data and produces an
//!    inherent that performs block rewards payouts or otherwise handles the data according to this particular
//!    Partner Chain's rules.
//! 6. The block participation pallet consumes the operational inherent data and cleans up block production data
//!    up to the slot limit.
//!
//! ## Usage
//!
//! To incorporate this feature into a Partner Chain, one must do the following:
//! 1. Implement a pallet consuming inherent data of type [BlockProductionData]
//! 2. Include the block participation pallet into their runtime and configure it. Consult the documentation of
//!    `pallet_block_participation` for details.
//! 3. Implement [BlockParticipationApi] for their runtime.
//! 4. Include [inherent_data::BlockParticipationInherentDataProvider] in their node's inherent data
//!    provider set for both proposal and verification of blocks.
//!
//! Configuring the pallet and implementing the runtime API requires there to be a source of block production data
//! present in the runtime that can be used by the feature. The intended source is `pallet_block_production_log` but
//! in principle anu pallet offering a similar interfaces can be used. An example of runtime API implementation using
//! the block participation log pallet looks like the following:
//! ```rust,ignore
//!	impl sp_block_participation::BlockParticipationApi<Block, BlockAuthor> for Runtime {
//!		fn should_release_data(slot: Slot) -> Option<Slot> {
//!			BlockParticipationPallet::should_release_data(slot)
//!		}
//!		fn blocks_produced_up_to_slot(slot: Slot) -> Vec<(Slot, BlockAuthor)> {
//!			<Runtime as pallet_block_participation::Config>::blocks_produced_up_to_slot(slot).collect()
//!		}
//!		fn target_inherent_id() -> InherentIdentifier {
//!			<Runtime as pallet_block_participation::Config>::TARGET_INHERENT_ID
//!		}
//!	}
//! ```
//!
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sidechain_domain::{DelegatorKey, MainchainKeyHash, McEpochNumber};
pub use sp_consensus_slots::{Slot, SlotDuration};
use sp_inherents::{InherentIdentifier, IsFatalError};

#[cfg(test)]
mod tests;

/// Inherent identifier used by the Block Participation pallet
///
/// This identifier is used for internal operation of the feature and is different from the target inherent ID
/// provided through [BlockParticipationApi].
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"blokpart";

/// Represents a block producer's delegator along with their number of shares in that block producer's pool.
///
/// Values of this type can only be interpreted in the context of their enclosing [BlockProducerParticipationData].
#[derive(
	Clone, Debug, PartialEq, Eq, Decode, DecodeWithMemTracking, Encode, TypeInfo, PartialOrd, Ord,
)]
pub struct DelegatorBlockParticipationData<DelegatorId> {
	/// Delegator Id
	pub id: DelegatorId,
	/// Number of this delegator's shares in the pool operated by the block producer of the enclosing [BlockProducerParticipationData].
	pub share: u64,
}

/// Aggregated data on block production of one block producer in one aggregation period.
///
/// Values of this type can only be interpreted in the context of their enclosing [BlockProductionData].
#[derive(
	Clone, Debug, PartialEq, Eq, Decode, DecodeWithMemTracking, Encode, TypeInfo, PartialOrd, Ord,
)]
pub struct BlockProducerParticipationData<BlockProducerId, DelegatorId> {
	/// Block producer ID
	pub block_producer: BlockProducerId,
	/// Number of block produced in the aggregation period represented by the current [BlockProducerParticipationData]
	pub block_count: u32,
	/// Total sum of shares of delegators in `delegators` field
	pub delegator_total_shares: u64,
	/// List of delegators of `block_producer` along with their share in the block producer's stake pool
	pub delegators: Vec<DelegatorBlockParticipationData<DelegatorId>>,
}

/// Aggregated data on block production, grouped by the block producer and aggregation period (main chain epoch).
///
/// When provided by the inherent data provider it should aggregate data since the previous `up_to_slot` to the current `up_to_slot`.
#[derive(Clone, Debug, PartialEq, Eq, Decode, DecodeWithMemTracking, Encode, TypeInfo)]
pub struct BlockProductionData<BlockProducerId, DelegatorId> {
	/// Data upper slot boundary.
	up_to_slot: Slot,
	/// Aggregated data on block producers and their delegators.
	///
	/// There may be more than one entry for the same block producer in this collection if the aggregated
	/// period spans multiple aggregation periods.
	producer_participation: Vec<BlockProducerParticipationData<BlockProducerId, DelegatorId>>,
}

impl<BlockProducerId, DelegatorId> BlockProductionData<BlockProducerId, DelegatorId> {
	/// Construct a new instance of [BlockProductionData], ensuring stable ordering of data.
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

	/// Returns the upper slot boundary of the aggregation range of `self`
	pub fn up_to_slot(&self) -> Slot {
		self.up_to_slot
	}

	/// Returns aggregated participation data per block producer
	pub fn producer_participation(
		&self,
	) -> &[BlockProducerParticipationData<BlockProducerId, DelegatorId>] {
		&self.producer_participation
	}
}

/// Error type returned by the Block Participation pallet's inherent
#[derive(Encode, PartialEq)]
#[cfg_attr(not(feature = "std"), derive(Debug))]
#[cfg_attr(
	feature = "std",
	derive(Decode, DecodeWithMemTracking, thiserror::Error, sp_runtime::RuntimeDebug)
)]
pub enum InherentError {
	/// Indicates that inherent was not produced when expected
	#[cfg_attr(feature = "std", error("Block participation inherent not produced when expected"))]
	InherentRequired,
	/// Indicates that inherent was produced when not expected
	#[cfg_attr(feature = "std", error("Block participation inherent produced when not expected"))]
	UnexpectedInherent,
	/// Indicates that the inherent was produced with incorrect slot boundary
	#[cfg_attr(feature = "std", error("Block participation up_to_slot incorrect"))]
	IncorrectSlotBoundary,
	/// Indicates that the inherent was produced with incorrect participation data
	#[cfg_attr(feature = "std", error("Inherent data provided by the node is invalid"))]
	InvalidInherentData,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

sp_api::decl_runtime_apis! {
	/// Runtime api exposing configuration and runtime bindings necessary for [inherent_data::BlockParticipationInherentDataProvider].
	///
	/// This API should typically be implemented by simply exposing relevant functions and data from the feature's pallet.
	pub trait BlockParticipationApi<BlockProducerId: Decode> {
		/// Returns slot up to which block production data should be released or [None].
		fn should_release_data(slot: Slot) -> Option<Slot>;
		/// Returns block authors since last processing up to `slot`.
		fn blocks_produced_up_to_slot(slot: Slot) -> Vec<(Slot, BlockProducerId)>;
		/// Returns the inherent ID under which block participation data should be provided.
		fn target_inherent_id() -> InherentIdentifier;
	}
}

/// Signifies that a type or some of its variants represents a Cardano stake pool operator
pub trait AsCardanoSPO {
	/// If [Self] represents a Cardano SPO, returns hash of this SPO's Cardano public key
	fn as_cardano_spo(&self) -> Option<MainchainKeyHash>;
}
impl AsCardanoSPO for Option<MainchainKeyHash> {
	fn as_cardano_spo(&self) -> Option<MainchainKeyHash> {
		*self
	}
}

/// Signifies that a type represents a Cardano delegator
pub trait CardanoDelegator {
	/// Converts a Cardano delegator key to [Self]
	fn from_delegator_key(key: DelegatorKey) -> Self;
}
impl<T: From<DelegatorKey>> CardanoDelegator for T {
	fn from_delegator_key(key: DelegatorKey) -> Self {
		key.into()
	}
}

/// Inherent data provider definitions and implementation for Block Producer feature
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
	use std::collections::HashMap;
	use std::hash::Hash;

	/// Cardano observability data source providing queries required by [BlockParticipationInherentDataProvider].
	#[async_trait::async_trait]
	pub trait BlockParticipationDataSource {
		/// Retrieves stake pool delegation distribution for provided epoch and pools
		async fn get_stake_pool_delegation_distribution_for_pools(
			&self,
			epoch: McEpochNumber,
			pool_hashes: &[MainchainKeyHash],
		) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>>;
	}

	/// Error returned by [BlockParticipationInherentDataProvider] constructors
	#[derive(thiserror::Error, Debug)]
	pub enum InherentDataCreationError<BlockProducerId: Debug> {
		/// Indicates that a runtime API failed
		#[error("Runtime API call failed: {0}")]
		ApiError(#[from] ApiError),
		/// Indicates that a data source call returned an error
		#[error("Data source call failed: {0}")]
		DataSourceError(Box<dyn Error + Send + Sync>),
		/// Indicates that Cardano stake delegation is missing for the epoch from which a block producer was selected
		///
		/// This error should never occur in normal operation of a node, unless the data source has been corrupted.
		#[error("Missing epoch {0} data for {1:?}")]
		DataMissing(McEpochNumber, BlockProducerId),
		/// Indicates that the Cardano epoch covering a producer block could not be computed while respecting the
		/// offset defined by [sidechain_domain::DATA_MC_EPOCH_OFFSET].
		///
		/// This error should never occur in normal operation of a node.
		#[error("Offset of {1} can not be applied to main chain epoch {0}")]
		McEpochBelowOffset(McEpochNumber, u32),
	}

	/// Inherent data provider for block participation data.
	/// This IDP is active only if the `BlockParticipationApi::should_release_data` function returns `Some`.
	/// This IDP provides two sets of inherent data:
	/// - One is the block production data saved under the inherent ID indicated by the function
	///   [BlockParticipationApi::target_inherent_id], which is intended for consumption by a chain-specific handler pallet.
	/// - The other is the slot limit returned by [BlockParticipationApi::should_release_data]. This inherent data
	///   is needed for internal operation of the feature and triggers clearing of already handled data
	///   from the block production log pallet.
	#[derive(Debug, Clone, PartialEq)]
	pub enum BlockParticipationInherentDataProvider<BlockProducerId, DelegatorId> {
		/// Active variant of the IDP that will provide inherent data stored in `block_production_data` at the
		/// inherent ID stored in `target_inherent_id`.
		Active {
			/// Inherent ID under which inherent data will be provided
			target_inherent_id: InherentIdentifier,
			/// Inherent data containing aggregated block participation data
			block_production_data: BlockProductionData<BlockProducerId, DelegatorId>,
		},
		/// Inactive variant of the IDP that will not provide any data and will not raise any errors
		Inert,
	}

	impl<BlockProducer, Delegator> BlockParticipationInherentDataProvider<BlockProducer, Delegator>
	where
		BlockProducer: AsCardanoSPO + Decode + Clone + Hash + Eq + Ord + Debug,
		Delegator: CardanoDelegator + Ord + Debug,
	{
		/// Creates a new inherent data provider of block participation data.
		///
		/// The returned inherent data provider will be inactive if [ProvideRuntimeApi] is not present
		/// in the runtime or if [BlockParticipationApi::should_release_data] returns [None].
		pub async fn new<Block: BlockT, T>(
			client: &T,
			data_source: &(dyn BlockParticipationDataSource + Send + Sync),
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

			if !api.has_api::<dyn BlockParticipationApi<Block, BlockProducer>>(parent_hash)? {
				return Ok(Self::Inert);
			}

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
			data_source: &(dyn BlockParticipationDataSource + Send + Sync),
		) -> Result<StakeDistribution, InherentDataCreationError<BlockProducer>> {
			let pools: Vec<_> = producers.flat_map(|p| p.as_cardano_spo()).collect();
			data_source
				.get_stake_pool_delegation_distribution_for_pools(mc_epoch, &pools)
				.await
				.map_err(InherentDataCreationError::DataSourceError)
		}

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

			offset_data_epoch(&mc_epoch)
				.map_err(|offset| InherentDataCreationError::McEpochBelowOffset(mc_epoch, offset))
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
				inherent_data.put_data(*target_inherent_id, block_production_data)?;
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
