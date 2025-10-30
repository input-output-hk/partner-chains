//! A Substrate pallet maintaining a consumable log of block production information.
//!
//! ## Purpose of this pallet
//!
//! This pallet keeps a log containing block producer IDs along with times of blocks produced by them.
//! This log is updated every block and is meant to consumed by other features.
//! The intended use of this pallet within the Partner Chains SDK is to expose block production data for consumption
//! by the Block Participation feature implemented by the `sp_block_participation` and `pallet_block_participation`
//! crates.
//!
//! ## Usage - PC Builder
//!
//! This pallet requires inherent data provided by the inherent data provider defined by `sp_block_production_log`
//! crate. Consult the crate's documentation for instruction on how to wire it into the node correctly.
//!
//! ### Adding to the runtime
//!
//! The pallet requires a minimal configuration. Consult the documentation for [pallet::Config] for details.
//!
//! An example configuration for a runtime using Aura consensus might look like this:
//!
//! ```rust,ignore
//! impl pallet_block_production_log::Config for Runtime {
//!     type BlockProducerId = BlockAuthor;
//!     type WeightInfo = pallet_block_production_log::weights::SubstrateWeight<Runtime>;
//!
//!     type Moment = u64;
//!
//!     #[cfg(feature = "runtime-benchmarks")]
//!     type BenchmarkHelper = PalletBlockProductionLogBenchmarkHelper;
//! }
//! ```
//!
//! #### Defining block producer ID
//!
//! The pallet expects the Partner Chain to provide a type representing its block producers.
//! This type can be as simple as an Aura public key but can also be a more complex type if block producers
//! are not a homogenous group. For example, in the context of a Partner Chain using Ariadne committee selection,
//! it's typical to have two kinds of block producers: permissioned producers provided by the governance authority
//! and registered candidates recruited from among Cardano stake pool operators. In this instance an example
//! author type could be:
//! ```rust
//! use sidechain_domain::*;
//!
//! pub enum BlockAuthor {
//!     Incentivized(CrossChainPublicKey, StakePoolPublicKey),
//!     ProBono(CrossChainPublicKey),
//! }
//! ```
//!
//! Keep in mind that other Partner Chains SDK components put their own constraints on the block author type
//! that need to be adhered to for a Partner Chain to integrated them.
//!
//! #### Support for adding to a running chain
//!
//! The pallet and its inherent data provider defined in [sp_block_production_log] are written in a way that allows for
//! adding it to an already live chain. The pallet allows for an initial period where inherent data is unavailable
//! and considers its inherent extrinsic required only after the first block where inherent data is provided.
//! Conversely, the inherent data provider is active only when the pallet and its runtime API is present for it to call.
//!
//! ### Consuming the log
//!
//! **Important**: Consuming the log is a destructive operation. Multiple features should not consume the log data
//!                unless they are coordinated to read and clear the same log prefix.
//!
//! The pallet exposes three functions that allow other pallets to consume its data: `take_prefix`, `peek_prefix`
//! and `drop_prefix`. Any feature using the log should be able to identify the time up to which it should
//! process the log data and either:
//! - call `take_prefix` from some pallet's logic and process the returned data within the same block
//! - call `peek_prefix` inside an inherent data provider and use `drop_prefix` from the corresponding pallet
//!   to clear the previously peeked data within the same block
//!
//! It is critically important to drop exactly the prefix processed to avoid either skipping or double-counting some blocks.
//!
//! ## Usage - PC user
//!
//! This pallet does not expose any user-facing functionalities.
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

pub mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_block_production_log::*;
	use sp_runtime::traits::Member;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// ID type that can represent any block producer in the network.
		/// This type should be defined by the Partner Chain Depending on its consensus mechanism and possible block producer types.
		type BlockProducerId: Member + Parameter + MaxEncodedLen;

		/// Weight information on extrinsic in the pallet. For convenience weights in [weights] module can be used.
		type WeightInfo: WeightInfo;

		/// Type used to identify the moment in time when the block was produced, eg. a timestamp or slot number.
		type Moment: Member + Parameter + MaxEncodedLen + PartialOrd + Ord + PartialEq + Eq;

		#[cfg(feature = "runtime-benchmarks")]
		/// Benchmark helper type used for running benchmarks
		type BenchmarkHelper: benchmarking::BenchmarkHelper<Self::BlockProducerId>;
	}

	#[pallet::storage]
	#[pallet::unbounded]
	pub type Log<T: Config> = StorageValue<_, Vec<(T::Moment, T::BlockProducerId)>, ValueQuery>;

	/// Temporary storage of the current block's producer, to be appended to the log on block finalization.
	#[pallet::storage]
	pub type CurrentProducer<T: Config> =
		StorageValue<_, (T::Moment, T::BlockProducerId), OptionQuery>;

	/// This storage is used to prevent calling `append` multiple times for the same block or for past blocks.
	#[pallet::storage]
	pub type LatestBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			Self::decode_inherent_data(data).unwrap().map(|data| Call::append {
				moment: data.moment,
				block_producer_id: data.block_producer_id,
			})
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::append { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			let has_data = Self::decode_inherent_data(data)?.is_some();
			if has_data || LatestBlock::<T>::get().is_some() {
				Ok(Some(Self::Error::InherentRequired))
			} else {
				Ok(None)
			}
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Call is not allowed, because the log has been already written for a block with same or higher number.
		BlockNumberNotIncreased,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedules an entry to be appended to the log. Log has to be ordered by a moment and writing the same moment twice is forbidden.
		#[pallet::call_index(0)]
		#[pallet::weight((T::WeightInfo::append(), DispatchClass::Mandatory))]
		pub fn append(
			origin: OriginFor<T>,
			moment: T::Moment,
			block_producer_id: T::BlockProducerId,
		) -> DispatchResult {
			ensure_none(origin)?;

			let current_block = <frame_system::Pallet<T>>::block_number();
			match LatestBlock::<T>::get() {
				Some(b) if b >= current_block => Err(Error::<T>::BlockNumberNotIncreased),
				_ => Ok(()),
			}?;
			LatestBlock::<T>::put(current_block);

			Ok(CurrentProducer::<T>::put((moment, block_producer_id)))
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// A dummy `on_initialize` to return the amount of weight that `on_finalize` requires to
		/// execute.
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			T::WeightInfo::on_finalize()
		}

		fn on_finalize(block: BlockNumberFor<T>) {
			if let Some((moment, block_producer_id)) = CurrentProducer::<T>::take() {
				log::info!("👷 Block {block:?} producer is {block_producer_id:?}");
				Log::<T>::append((moment, block_producer_id));
			} else {
				log::warn!(
					"👷 Block {block:?} producer not set. This should occur only at the beginning of the production log pallet's lifetime."
				)
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn decode_inherent_data(
			data: &InherentData,
		) -> Result<
			Option<BlockProductionInherentDataV1<T::Moment, T::BlockProducerId>>,
			InherentError,
		> {
			data.get_data::<BlockProductionInherentDataV1<T::Moment, T::BlockProducerId>>(
				&Self::INHERENT_IDENTIFIER,
			)
			.map_err(|_| InherentError::InvalidInherentData)
		}

		/// Returns all entries up to `moment` (inclusive) and removes them from the log
		pub fn take_prefix(moment: &T::Moment) -> Vec<(T::Moment, T::BlockProducerId)> {
			let removed_prefix = Log::<T>::mutate(|log| {
				let pos = log.partition_point(|(s, _)| s <= moment);
				log.drain(..pos).collect()
			});
			removed_prefix
		}

		/// Returns all entries up to `moment` (inclusive) from the log
		pub fn peek_prefix(
			moment: &T::Moment,
		) -> impl Iterator<Item = (T::Moment, T::BlockProducerId)> {
			Log::<T>::get().into_iter().take_while(move |(s, _)| s <= moment)
		}

		/// Removes all entries up to `moment` (inclusive) from the log
		pub fn drop_prefix(moment: &T::Moment) {
			Log::<T>::mutate(|log| {
				let position = log.partition_point(|(s, _)| s <= moment);
				log.drain(..position);
			});
		}
	}
}

#[cfg(feature = "block-participation")]
mod block_participation {
	use pallet_block_participation::BlockParticipationProvider;

	impl<T: crate::Config> BlockParticipationProvider<T::Moment, T::BlockProducerId>
		for crate::Pallet<T>
	{
		fn blocks_to_process(
			moment: &T::Moment,
		) -> impl Iterator<Item = (T::Moment, T::BlockProducerId)> {
			Self::peek_prefix(moment)
		}

		fn discard_processed_blocks(moment: &T::Moment) {
			Self::drop_prefix(moment)
		}
	}
}
