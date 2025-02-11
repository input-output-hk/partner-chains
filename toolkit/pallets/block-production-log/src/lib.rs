#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
pub mod test;
pub mod weights;

pub use pallet::*;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_block_production_log::*;
	use sp_consensus_slots::Slot;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type BlockProducerId: Member + Parameter + MaxEncodedLen;

		type WeightInfo: WeightInfo;

		fn current_slot() -> Slot;
	}

	#[pallet::storage]
	#[pallet::unbounded]
	pub type Log<T: Config> = StorageValue<_, Vec<(Slot, T::BlockProducerId)>, ValueQuery>;

	/// This storage is used to prevent calling `append` multiple times for the same block or for past blocks.
	#[pallet::storage]
	pub type LatestBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let block_producer_id = data
				.get_data::<T::BlockProducerId>(&Self::INHERENT_IDENTIFIER)
				.expect("Block Production Log inherent data not correctly encoded")
				.expect("Block Production Log inherent data must be provided");
			Some(Call::append { block_producer_id })
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::append { .. })
		}

		fn is_inherent_required(_: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			Ok(Some(Self::Error::InherentRequired))
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Call is not allowed, because the log has been already written for a block with same or higher number.
		BlockNumberNotIncreased,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Appends an entry to the log. Log has to be ordered by slots and writing the same slot twice is forbidden.
		#[pallet::call_index(0)]
		#[pallet::weight((T::WeightInfo::append(), DispatchClass::Mandatory))]
		pub fn append(
			origin: OriginFor<T>,
			block_producer_id: T::BlockProducerId,
		) -> DispatchResult {
			ensure_none(origin)?;

			let current_block = <frame_system::Pallet<T>>::block_number();
			match LatestBlock::<T>::get() {
				Some(b) if b >= current_block => Err(Error::<T>::BlockNumberNotIncreased),
				_ => Ok(()),
			}?;
			LatestBlock::<T>::put(current_block);

			Ok(Log::<T>::append((T::current_slot(), block_producer_id)))
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn take_prefix(slot: &Slot) -> Vec<(Slot, T::BlockProducerId)> {
			let (to_return, to_retain) = Log::<T>::get().into_iter().partition(|(s, _)| s <= slot);
			Log::<T>::put(to_retain);
			to_return
		}
	}
}
