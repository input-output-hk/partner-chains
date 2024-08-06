#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;
use sidechain_domain::McBlockHash;
use sp_core::{Decode, Encode};
use sp_inherents::IsFatalError;

#[derive(Encode, sp_runtime::RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Decode))]
pub enum InherentError {
	DoesNotMatchInherentData { expected: McBlockHash, actual: McBlockHash },
	ChangeNotHandled,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sidechain_domain::{McBlockHash, ScEpochNumber, ScSlotNumber};
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
	#[pallet::getter(fn epoch_number)]
	pub(super) type EpochNumber<T: Config> = StorageValue<_, ScEpochNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn slots_per_epoch)]
	pub(super) type SlotsPerEpoch<T: Config> =
		StorageValue<_, sidechain_slots::SlotsPerEpoch, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn last_mc_hash)]
	pub(super) type LastMcHash<T: Config> =
		StorageValue<_, sidechain_domain::McBlockHash, OptionQuery>;

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

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = crate::InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = sidechain_mc_hash::INHERENT_IDENTIFIER;

		/// Responsible for calling `Call:set()` on each block by the block author, if the validator list changed
		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let mc_hash = Self::get_mc_hash_from_inherent_data(data);

			match LastMcHash::<T>::get() {
				None => Some(Call::set_last_mc_hash { mc_hash }),
				Some(last_mc_hash) if last_mc_hash == mc_hash => None,
				Some(_) => Some(Call::set_last_mc_hash { mc_hash }),
			}
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			if let Self::Call::set_last_mc_hash { mc_hash: expected } = call {
				let actual = Self::get_mc_hash_from_inherent_data(data);
				if *expected != actual {
					return Err(Self::Error::DoesNotMatchInherentData {
						expected: expected.clone(),
						actual,
					});
				}
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::set_last_mc_hash { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			let mc_hash = Self::get_mc_hash_from_inherent_data(data);
			match LastMcHash::<T>::get() {
				Some(last_mc_hash) if last_mc_hash != mc_hash => {
					Ok(Some(Self::Error::ChangeNotHandled))
				},
				_ => Ok(None),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn get_mc_hash_from_inherent_data(data: &InherentData) -> sidechain_domain::McBlockHash {
			data.get_data::<McBlockHash>(&Self::INHERENT_IDENTIFIER)
				.expect("❌ MC Block Hash inherent data is invalid")
				.expect("❌ MC Block Hash inherent data is missing")
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight((1, DispatchClass::Mandatory))]
		pub fn set_last_mc_hash(
			origin: OriginFor<T>,
			mc_hash: sidechain_domain::McBlockHash,
		) -> DispatchResult {
			ensure_none(origin)?;
			log::info!("#️⃣ New MC block referenced: {mc_hash}");
			LastMcHash::<T>::put(mc_hash);
			Ok(())
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
