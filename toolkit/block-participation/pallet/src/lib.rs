//! Pallet providing configuration and supporting runtime logic for the block participation data feature of Partner Chains SDK.
//!
//! ## Purpose of this pallet
//!
//! This pallet provides the runtime-side logic supporting the block participation data feature of PC SDK.
//! Unlike most pallets, this one is not meant to be interacted with either by the chain's users or other
//! runtime components in the system. Instead, it only serves two purposes:
//! - it provides all configuration required by the feature's inherent data provider defined in the primitives crate
//! - it provides an inhrenent extrinsic that removes from the runtime storage data that has been already
//!   processed by the inherent data provider
//! The reason for that is that the feature's purpose is to produce inherent data containing block participation
//! data for consumption by custom-written pallet provided by each Partner Chain itself.
//!
//! The pallet is expected to be used together with the `pallet_block_production_log` when deployed in the
//! context of Partner Chains SDK.
//!
//! ## Usage
//!
//! ### Adding into the runtime
//!
//! Consult documentation of [pallet::Config] for details on each configuration field.
//!
//! Assuming that the runtime also contains the `pallet_block_production_log`, an example configuration of
//! the pallet might look like the following:
//! ```rust,ignore
//! const RELEASE_PERIOD: u64 = 128;
//!
//! impl pallet_block_participation::Config for Runtime {
//!     type WeightInfo = pallet_block_participation::weights::SubstrateWeight<Runtime>;
//!     type BlockAuthor = BlockAuthor;
//!     type DelegatorId = DelegatorKey;
//!
//!     type BlockParticipationProvider = BlockProductionLog;
//!
//!     const TARGET_INHERENT_ID: InherentIdentifier = *b"_example";
//! }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use frame_support::pallet_prelude::*;
pub use pallet::*;
use sp_block_participation::*;

/// Source of block participation data
pub trait BlockParticipationProvider<Moment, BlockProducer> {
	/// Returns the block data for processing
	fn blocks_to_process(moment: &Moment) -> impl Iterator<Item = (Moment, BlockProducer)>;

	/// Discards processed data
	fn discard_processed_blocks(moment: &Moment);
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Weight info for this pallet's extrinsics
		type WeightInfo: crate::weights::WeightInfo;

		/// Moment in time at which the participation data should be processed
		///
		/// This type should be convertible to a timestamp value. If it represents a time range,
		/// a representative timestamp, such as the start of the range should be computable from it.
		type Moment: Parameter + Default + MaxEncodedLen + PartialOrd;

		/// Source of block participation data
		///
		/// The default implementation provided by the Partner Chains toolit is the block production
		/// log pallet implemented by the `pallet_block_production_log` crate.
		type BlockParticipationProvider: BlockParticipationProvider<Self::Moment, Self::BlockAuthor>;

		/// Type identifying the producer of a block on the Partner Chain
		type BlockAuthor: Member + Parameter + MaxEncodedLen;

		/// Type identifying indirect block production participants on the Partner Chain
		/// This can be native stakers on Partner Chain, stakers on the main chain or other.
		type DelegatorId: Member + Parameter + MaxEncodedLen;

		/// Inherent ID under which block participation data should be provided.
		/// It should be set to the ID used by the pallet that will process participation data for
		/// paying out block rewards or other purposes.
		const TARGET_INHERENT_ID: InherentIdentifier;
	}

	#[pallet::storage]
	pub type ProcessedUpTo<T: Config> = StorageValue<_, T::Moment, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		///sss
		MomentNotIncreasing,
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = sp_block_participation::InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = sp_block_participation::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let up_to_moment = Self::decode_inherent_data(data).unwrap()?;
			Some(Call::note_processing { up_to_moment })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Some(expected_moment) = Self::decode_inherent_data(data)? else {
				return Err(Self::Error::UnexpectedInherent);
			};

			let Self::Call::note_processing { up_to_moment } = call else {
				unreachable!("There should be no other extrinsic in the pallet")
			};

			ensure!(expected_moment == *up_to_moment, Self::Error::InvalidInherentData);

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::note_processing { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			if Self::decode_inherent_data(data)?.is_some() {
				Ok(Some(Self::Error::InherentRequired))
			} else {
				Ok(None)
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn decode_inherent_data(data: &InherentData) -> Result<Option<T::Moment>, InherentError> {
			data.get_data(&Self::INHERENT_IDENTIFIER)
				.map_err(|_| InherentError::InvalidInherentData)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registers the fact that block participation data has been released for processing
		/// and removes the handled data from block production log.
		///
		/// This inherent does not by itself process any data and only serves an operational function
		/// by cleaning up data that has been already processed by other components.
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn note_processing(origin: OriginFor<T>, up_to_moment: T::Moment) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(ProcessedUpTo::<T>::get() < up_to_moment, Error::<T>::MomentNotIncreasing);
			log::info!("🧾 Processing block participation data");
			T::BlockParticipationProvider::discard_processed_blocks(&up_to_moment);
			ProcessedUpTo::<T>::set(up_to_moment);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Fetches all blocks to be processed
		pub fn blocks_to_process(moment: &T::Moment) -> Vec<(T::Moment, T::BlockAuthor)> {
			<T as Config>::BlockParticipationProvider::blocks_to_process(moment).collect()
		}

		/// Discards processed data
		pub fn discard_processed_blocks(moment: &T::Moment) {
			T::BlockParticipationProvider::discard_processed_blocks(moment);
		}

		/// Returns the inherent ID at which the participation feature should provide participation data
		pub fn target_inherent_id() -> InherentIdentifier {
			<T as Config>::TARGET_INHERENT_ID
		}
	}
}
