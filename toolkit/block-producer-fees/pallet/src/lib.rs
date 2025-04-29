//! Pallet to store Block Producer Fees settings that are relevant to rewards payments.
//!
//! Margin fee is percent of block rewards that will be paid to the block producer before
//! distributing the rest of rewards to his stakers. Precision of the margin fee setting is bounded
//! to 1/100 of a percent.
//!
//! The margin fees are stored together with the slot at which change occurred, so this data can be
//! exposed to rewards calculation.
//!
//! Log of changes per account is bounded. The oldest entries are dropped when new ones are added.
//! Intention is to discourage users from too frequent changes and there is an assumption
//! that rewards calculation algorithm will account for it.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_consensus_slots::Slot;
	use sp_std::collections::vec_deque::VecDeque;

	/// Current version of the pallet
	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Maximum number of past changes per one block producer kept in the storage.
		type HistoricalChangesPerProducer: Get<u16>;

		/// Weight information on extrinsic in the pallet. For convenience weights in [weights] module can be used.
		type WeightInfo: WeightInfo;

		/// The slot number of the current block.
		fn current_slot() -> Slot;
	}

	// Margin Fee precision is 0.01 of a percent, so use 1/10000 as unit.
	type PerTenThousands = u16;

	type FeeChange = (Slot, PerTenThousands);

	/// Stores bounded amount of fee changes per
	#[pallet::storage]
	#[pallet::unbounded]
	pub type FeesChanges<T: Config> = StorageMap<
		Hasher = Twox64Concat,
		Key = T::AccountId,
		Value = VecDeque<FeeChange>,
		QueryKind = ValueQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets the margin fee of a caller. Margin fee is (fee numerator / 10000).
		#[pallet::call_index(0)]
		#[pallet::weight((T::WeightInfo::set_fee(), DispatchClass::Normal))]
		pub fn set_fee(origin: OriginFor<T>, fee_numerator: PerTenThousands) -> DispatchResult {
			let account_id = ensure_signed(origin)?;
			if fee_numerator > 10000 {
				return Err(DispatchError::Other("fee numerator must be in range from 0 to 10000"));
			}
			FeesChanges::<T>::mutate(account_id, |fees_log| {
				if fees_log.len() > T::HistoricalChangesPerProducer::get().into() {
					let _ = fees_log.pop_back();
				}
				fees_log.push_front((T::current_slot(), fee_numerator));
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the current pallet version.
		pub fn get_version() -> u32 {
			PALLET_VERSION
		}

		/// Retrieves all stored block producer fees settings.
		pub fn get_all() -> impl Iterator<Item = (T::AccountId, VecDeque<FeeChange>)> {
			FeesChanges::<T>::iter()
		}

		/// Retrieves the latest fee settings for all accounts.
		pub fn get_all_latest() -> impl Iterator<Item = (T::AccountId, FeeChange)> {
			Self::get_all().map(|(account_id, changes)| {
				(account_id, *changes.front().expect("There are no empty collections in storage"))
			})
		}

		/// Retrieves block producers fees settings.
		pub fn get(id: T::AccountId) -> VecDeque<FeeChange> {
			FeesChanges::<T>::get(id)
		}

		/// Gets the latest fee setting for the given account.
		pub fn get_latest(id: T::AccountId) -> Option<FeeChange> {
			FeesChanges::<T>::get(id).front().cloned()
		}
	}
}
