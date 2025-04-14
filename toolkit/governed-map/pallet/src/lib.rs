//! Pallet exposing the key-value pairs sourced from the Cardano ledger as part of the Governed Map feature.
//!
//! # Purpose of this pallet
//!
//! This pallet stores the most recent state of the key-value pairs in the Governed Map on Cardano for use
//! by other runtime components. It also exposes hooks for other components to be notified when particular
//! key-value pair is inserted, updated or deleted.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use frame_system::WeightInfo;
use sidechain_domain::byte_string::*;
use sp_governed_map::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub const PALLET_VERSION: u32 = 1;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Maximum number of changes that can be registered in a single inherent.
		///
		/// This value should be high enough for all changes to be registered in one block.
		/// Setting this to a value higher than the total number of parameters in the Governed Map guarantees that.
		type MaxChanges: Get<u32>;

		/// Maximum length of the key in the Governed Map in bytes.
		type MaxKeyLength: Get<u32>;

		/// Maximum length of data stored under a single key in the Governed Map
		type MaxValueLength: Get<u32>;

		// TODO benchmarks
		type WeightInfo: WeightInfo;
	}

	pub(crate) type MapKey<T> = BoundedString<<T as Config>::MaxKeyLength>;
	pub(crate) type MapValue<T> = BoundedVec<u8, <T as Config>::MaxValueLength>;

	/// Stores the latest state of the Governed Map that was observed on Cardano.
	#[pallet::storage]
	pub type Mapping<T: Config> = StorageMap<_, Twox64Concat, MapKey<T>, MapValue<T>, OptionQuery>;

	/// Cardano address of the Governed Map validator.
	///
	/// This address is used by the observability component to query current state of the mapping
	#[pallet::storage]
	pub type MainChainScripts<T: Config> = StorageValue<_, MainChainScriptsV1, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Initial address of the Governed Map validator.
		///
		/// If it is left empty, the Governance Map pallet will be inactive until the address is set via extrinsic.
		pub main_chain_script: Option<MainChainScriptsV1>,
		pub _marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MainChainScripts::<T>::set(self.main_chain_script.clone());
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			Self::create_inherent_or_err(data).expect("Creating Governed Map inherent failed")
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Some(expected_call) = Self::create_inherent(data) else {
				return Err(Self::Error::InherentNotExpected);
			};

			if *call != expected_call {
				return Err(Self::Error::IncorrectInherent);
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::register_changes { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			match Self::decode_inherent_data(data) {
				None => Ok(None),
				Some(_) => Ok(Some(Self::Error::InherentMissing)),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn decode_inherent_data(data: &InherentData) -> Option<Vec<GovernedMapChangeV1>> {
			data.get_data::<Vec<GovernedMapChangeV1>>(&INHERENT_IDENTIFIER)
				.expect("Governed Map inherent data is not encoded correctly")
		}

		fn create_inherent_or_err(data: &InherentData) -> Result<Option<Call<T>>, InherentError> {
			use InherentError::*;

			let Some(raw_changes) = Self::decode_inherent_data(data) else { return Ok(None) };

			if raw_changes.len() > T::MaxChanges::get() as usize {
				return Err(TooManyChanges);
			}

			let mut changes: BoundedVec<(MapKey<T>, Option<MapValue<T>>), T::MaxChanges> =
				BoundedVec::new();
			for GovernedMapChangeV1 { key, new_value } in raw_changes {
				let new_value = match new_value {
					Some(new_value) => Some(Self::bound_value(&key, new_value)?),
					None => None,
				};
				let key = Self::bound_key(&key)?;
				changes.try_push((key, new_value)).expect("Number of changes is below maximum");
			}
			Ok(Some(Call::register_changes { changes }))
		}

		fn bound_key(key: &str) -> Result<MapKey<T>, InherentError> {
			key.try_into().map_err(|_| InherentError::KeyExceedsBounds(key.into()))
		}

		fn bound_value(key: &str, value: ByteString) -> Result<MapValue<T>, InherentError> {
			(value.0.clone().try_into())
				.map_err(|_| InherentError::ValueExceedsBounds(key.into(), value))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Inherent to register any changes in the state of the Governed Map on Cardano compared to the state currently stored in the pallet.
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn register_changes(
			origin: OriginFor<T>,
			changes: BoundedVec<(MapKey<T>, Option<MapValue<T>>), T::MaxChanges>,
		) -> DispatchResult {
			ensure_none(origin)?;

			log::info!("💾 Registering {} Governed Map changes", changes.len(),);

			for (key, value) in changes {
				Mapping::<T>::set(key, value);
			}

			Ok(())
		}

		/// Changes the address of the Governed Map validator used for observation.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		#[pallet::call_index(1)]
		#[pallet::weight((0, DispatchClass::Normal))]
		pub fn set_main_chain_scripts(
			origin: OriginFor<T>,
			new_main_chain_script: MainChainScriptsV1,
		) -> DispatchResult {
			ensure_root(origin)?;
			MainChainScripts::<T>::put(new_main_chain_script.clone());
			log::info!("🗂️ Governed Map main chain scripts updated to {new_main_chain_script:?}");
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the value under `key` or [None] otherwise.
		pub fn get_key_value(key: MapKey<T>) -> Option<BoundedVec<u8, T::MaxValueLength>> {
			Mapping::<T>::get(key)
		}

		/// Returns an iterator over all key-value pairs in the pallet storage.
		pub fn get_all_key_value_pairs() -> impl Iterator<Item = (MapKey<T>, MapValue<T>)> {
			Mapping::<T>::iter()
		}

		/// Returns current pallet version.
		pub fn get_version() -> u32 {
			PALLET_VERSION
		}
	}
}
