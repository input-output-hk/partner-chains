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
//! data for consumption by constom-written pallet provided by each Partner Chain itself.
//!
//! The pallet is expected to be used together with the [pallet_block_production_log] when deployed in the
//! context of Partner Chains SDK.
//!
//! ## Usage
//!
//! ### Adding into the runtime
//!
//! The pallet's configuration can be divided into three groups by purpose:
//! - `BlockAuthor` and `DelegatorId` types representing block authors and their dependant block beneficiaries
//! - `should_release_data` function that controls when the inherent data provider is active
//! - `blocks_produced_up_to_slot` and `blocks_produced_upd_to_slot` functions that provide bindings for consuming
//!   (reading and clearing) block production data. Most easily these should come from [pallet_block_production_log].
//!
//! Consult documentation of [pallet::Config] for details on each configuration field.
//!
//! Assumming that the runtime also contains the `pallet_block_production_log`, an example configuration of
//! the pallet might look like the following:
//! ```rust,ignore
//! const RELEASE_PERIOD: u64 = 128;
//!
//! impl pallet_block_participation::Config for Runtime {
//!     type WeightInfo = pallet_block_participation::weights::SubstrateWeight<Runtime>;
//!     type BlockAuthor = BlockAuthor;
//!     type DelegatorId = DelegatorKey;
//!
//!     // release data every `RELEASE_PERIOD` blocks, up to current slot
//!     fn should_release_data(slot: sidechain_slots::Slot) -> Option<sidechain_slots::Slot> {
//!         if System::block_number() % RELEASE_PERIOD == 0 {
//!             Some(slot)
//!         } else {
//!             None
//!         }
//!     }
//!
//!     fn blocks_produced_up_to_slot(slot: Slot) -> impl Iterator<Item = (Slot, BlockAuthor)> {
//!         BlockProductionLog::peek_prefix(slot)
//!     }
//!
//!     fn discard_blocks_produced_up_to_slot(slot: Slot) {
//!         BlockProductionLog::drop_prefix(&slot)
//!     }
//!
//!     const TARGET_INHERENT_ID: InherentIdentifier = *b"_example";
//! }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]

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

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type WeightInfo: crate::weights::WeightInfo;

		/// Type identifying the producer of a block on the Partner Chain
		type BlockAuthor: Member + Parameter + MaxEncodedLen;

		/// Type identifying indirect block production participants on the Partner Chain
		/// This can be native stakers on Partner Chain, stakers on the main chain or other.
		type DelegatorId: Member + Parameter + MaxEncodedLen;

		/// Should return slot up to which block production data should be released or None.
		fn should_release_data(slot: Slot) -> Option<Slot>;

		/// Returns block authors since last processing up to `slot`
		fn blocks_produced_up_to_slot(
			slot: Slot,
		) -> impl Iterator<Item = (Slot, Self::BlockAuthor)>;

		/// Discards block production data at the source up to slot
		/// This should remove exactly the same data as returned by `blocks_produced_up_to_slot`
		fn discard_blocks_produced_up_to_slot(slot: Slot);

		/// Inherent ID under which block participation data should be provided.
		/// It should be set to the ID used by the pallet that will process participation data for
		/// paying out block rewards or other purposes.
		const TARGET_INHERENT_ID: InherentIdentifier;
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = sp_block_participation::InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = sp_block_participation::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			// we unwrap here because we can't continue proposing a block if inherent data is invalid for some reason
			let up_to_slot = Self::decode_inherent_data(data).unwrap()?;

			Some(Call::note_processing { up_to_slot })
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Some(expected_inherent_data) = Self::decode_inherent_data(data)? else {
				return Err(Self::Error::UnexpectedInherent);
			};

			let Self::Call::note_processing { up_to_slot } = call else {
				unreachable!("There should be no other extrinsic in the pallet")
			};

			ensure!(*up_to_slot == expected_inherent_data, Self::Error::IncorrectSlotBoundary);

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
		fn decode_inherent_data(data: &InherentData) -> Result<Option<Slot>, InherentError> {
			data.get_data(&Self::INHERENT_IDENTIFIER)
				.map_err(|_| InherentError::InvalidInherentData)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registers the fact that block participation data has been released and removes the handled data from block production log.
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn note_processing(origin: OriginFor<T>, up_to_slot: Slot) -> DispatchResult {
			ensure_none(origin)?;
			log::info!("🧾 Processing block participation data up to slot {}.", *up_to_slot);
			T::discard_blocks_produced_up_to_slot(up_to_slot);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns slot up to which block production data should be released or [None].
		pub fn should_release_data(slot: Slot) -> Option<Slot> {
			<T as Config>::should_release_data(slot)
		}
	}
}
