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

	/// Temporary storage of the current block's producer, to be appended to the log on block finalization.
	#[pallet::storage]
	pub type CurrentProducer<T: Config> = StorageValue<_, T::BlockProducerId, OptionQuery>;

	/// This storage is used to prevent calling `append` multiple times for the same block or for past blocks.
	#[pallet::storage]
	pub type LatestBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// This flag keeps track of whether the pallet has been initialized by receiving non-empty inherent data.
	/// In initialized state, the inherent is requires for every block.
	#[pallet::storage]
	pub type Initialized<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			Self::decode_inherent_data(data)
				.unwrap()
				.map(|block_producer_id| Call::append { block_producer_id })
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::append { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			let has_data = Self::decode_inherent_data(data)?.is_some();
			if has_data || Initialized::<T>::get() {
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
		/// Schedules an entry to be appended to the log. Log has to be ordered by slots and writing the same slot twice is forbidden.
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

			Ok(CurrentProducer::<T>::put(block_producer_id))
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(block: BlockNumberFor<T>) {
			if let Some(block_producer_id) = CurrentProducer::<T>::take() {
				log::info!("👷 Block {block:?} producer is {block_producer_id:?}");
				Log::<T>::append((T::current_slot(), block_producer_id));
				Initialized::<T>::put(true);
			} else {
				log::warn!("👷 Block {block:?} producer not set. This should occur only at the beginning of the production log pallet's lifetime.")
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn decode_inherent_data(
			data: &InherentData,
		) -> Result<Option<T::BlockProducerId>, InherentError> {
			data.get_data::<T::BlockProducerId>(&Self::INHERENT_IDENTIFIER)
				.map_err(|_| InherentError::InvalidData)
		}
		pub fn take_prefix(slot: &Slot) -> Vec<(Slot, T::BlockProducerId)> {
			let removed_prefix = Log::<T>::mutate(|log| {
				let pos = log.partition_point(|(s, _)| s <= slot);
				log.drain(..pos).collect()
			});
			removed_prefix
		}

		pub fn peek_prefix(slot: Slot) -> impl Iterator<Item = (Slot, T::BlockProducerId)> {
			Log::<T>::get().into_iter().take_while(move |(s, _)| s <= &slot)
		}

		pub fn drop_prefix(slot: &Slot) {
			Log::<T>::mutate(|log| {
				let position = log.partition_point(|(s, _)| s <= slot);
				log.drain(..position);
			});
		}
	}
}
