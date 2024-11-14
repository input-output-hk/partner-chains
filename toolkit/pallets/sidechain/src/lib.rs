#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::BlockNumberFor;
	use sidechain_domain::{ScEpochNumber, ScSlotNumber};
	use sp_sidechain::OnNewEpoch;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		fn current_slot_number() -> ScSlotNumber;
		type OnNewEpoch: OnNewEpoch;

		/// Set of parameters that configure and identify the chain.
		type SidechainParams: Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ Clone
			+ Default;
	}

	#[pallet::storage]
	pub(super) type EpochNumber<T: Config> = StorageValue<_, ScEpochNumber, ValueQuery>;

	#[pallet::storage]
	pub(super) type SlotsPerEpoch<T: Config> =
		StorageValue<_, sidechain_slots::SlotsPerEpoch, ValueQuery>;

	#[pallet::storage]
	pub(super) type SidechainParams<T: Config> = StorageValue<_, T::SidechainParams, ValueQuery>;

	impl<T: Config> Pallet<T> {
		pub fn sidechain_params() -> T::SidechainParams {
			SidechainParams::<T>::get()
		}

		pub fn current_epoch_number() -> ScEpochNumber {
			let current_slot = T::current_slot_number();
			let slots_per_epoch = Self::slots_per_epoch();
			slots_per_epoch.epoch_number_from_sc_slot(current_slot)
		}

		pub fn slots_per_epoch() -> sidechain_slots::SlotsPerEpoch {
			SlotsPerEpoch::<T>::get()
		}
	}

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub params: T::SidechainParams,
		pub slots_per_epoch: sidechain_slots::SlotsPerEpoch,
		#[serde(skip)]
		pub _config: sp_std::marker::PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			SidechainParams::<T>::put(self.params.clone());
			SlotsPerEpoch::<T>::put(self.slots_per_epoch);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let real_epoch = Self::current_epoch_number();

			match EpochNumber::<T>::try_get().ok() {
				Some(saved_epoch) if saved_epoch != real_epoch => {
					log::info!("⏳ New epoch {real_epoch} starting at block {:?}", n);
					EpochNumber::<T>::put(real_epoch);
					<T::OnNewEpoch as OnNewEpoch>::on_new_epoch(saved_epoch, real_epoch)
						.saturating_add(T::DbWeight::get().reads_writes(2, 1))
				},
				None => {
					log::info!("⏳ Initial epoch {real_epoch} starting at block {:?}", n);
					EpochNumber::<T>::put(real_epoch);
					T::DbWeight::get().reads_writes(2, 1)
				},
				_ => T::DbWeight::get().reads_writes(2, 0),
			}
		}
	}
}
