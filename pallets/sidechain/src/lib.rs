#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::ensure_root;
	use frame_system::pallet_prelude::*;
	use sidechain_domain::{ScEpochNumber, ScSlotNumber, UtxoId};
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

		type MainChainScripts: Member + Parameter + MaybeSerializeDeserialize + MaxEncodedLen;

		fn set_main_chain_scripts(scripts: Self::MainChainScripts);
	}

	#[pallet::storage]
	pub(super) type EpochNumber<T: Config> = StorageValue<_, ScEpochNumber, ValueQuery>;

	#[pallet::storage]
	pub(super) type SlotsPerEpoch<T: Config> =
		StorageValue<_, sidechain_slots::SlotsPerEpoch, ValueQuery>;

	#[pallet::storage]
	pub(super) type SidechainParams<T: Config> = StorageValue<_, T::SidechainParams, ValueQuery>;

	#[pallet::storage]
	pub(super) type GenesisUtxo<T: Config> = StorageValue<_, UtxoId, ValueQuery>;

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

	/// Priviledged extrinsic to atomically upgrade runtime code and vital sidechain parameters.
	///
	/// Parameters:
	/// - `code`: WASM of the new runtime
	/// - `genesis_utxo`: genesis utxo burned by the `init-governance` transaction
	/// - `main_chain_scripts`: policies and addresses obtained from the `addresses` for the `genesis_utxo`
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(1)]
		#[pallet::weight((0, DispatchClass::Normal))]
		pub fn upgrade_and_set_addresses(
			origin: OriginFor<T>,
			code: sp_std::vec::Vec<u8>,
			genesis_utxo: UtxoId,
			main_chain_scripts: T::MainChainScripts,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin.clone())?;

			GenesisUtxo::<T>::set(genesis_utxo);

			T::set_main_chain_scripts(main_chain_scripts);

			// Runtime upgrade must be last because it consumes the rest of the block time
			frame_system::Pallet::<T>::set_code(origin, code)
		}
	}
}
