//! Pallet exposing key-value pairs sourced from the Cardano ledger as part of the Governed Map feature.
//!
//! # Purpose of this pallet
//!
//! This pallet stores the most recent state of the key-value pairs in the Governed Map on Cardano for use
//! by other runtime components. It also exposes hooks for other components to be notified when particular
//! key-value pair is inserted, updated or deleted.
//!
//! # Usage
//!
//! ## Adding to runtime
//!
//! ### Defining size limits
//!
//! Before adding the pallet to your runtime, first decide on the limits for the data that it will process:
//! - `MaxChanges`: the maximum number of changes that will be processed in one inherent invocation.
//!                 This number should be high enough to guarantee the inherent will always have the
//!                 capacity required to process incoming changes. Setting this limit above the expected
//!                 number of keys in use is a safe option.
//! - `MaxKeyLength`: maximum length of keys used. Be warned that if a key is set in the Governed Map
//!                   that exceeds this lenght limit, this pallet's inherent will fail and stall block
//!                   production, with the only recovery path being removal of this key so it is no longer
//!                   in the change set.
//! - `MaxValueLength`: maximum length of the value under a key. Same considerations as for `MaxKeyLength`
//!                     apply.
//! Once the limit values are decided, define them in your runtime, like so:
//! ```rust
//! frame_support::parameter_types! {
//!        pub const MaxChanges: u32 = 16;
//!        pub const MaxKeyLength: u32 = 64;
//!        pub const MaxValueLength: u32 = 512;
//! }
//! ```
//!
//! ### Implementing on-change handler
//!
//! The pallet allows the runtime implementer to define a handler that will be called for all key-value
//! changes registered by the pallet. If your runtime needs to react to changes, crate a type implementing
//! the [OnGovernedMappingChange] trait, eg:
//! ```rust
//! # use frame_support::BoundedVec;
//! # use sidechain_domain::byte_string::BoundedString;
//! # use sp_core::Get;
//! struct ChangeHandler;
//!
//! impl<MaxKeyLength, MaxValueLength> sp_governed_map::OnGovernedMappingChange<MaxKeyLength, MaxValueLength> for ChangeHandler
//! where
//!    MaxKeyLength: Get<u32>,
//!    MaxValueLength: Get<u32>,
//! {
//!    fn on_governed_mapping_change(
//!        key: BoundedString<MaxKeyLength>,
//!        new_value: Option<BoundedVec<u8, MaxValueLength>>,
//!        old_value: Option<BoundedVec<u8, MaxValueLength>>,
//!    ) {
//!        log::info!("Governed Map change for key {key}: old value: {old_value:?}, new value: {new_value:?}");
//!    }
//! }
//! ```
//! If any handling is not needed, a no-op implementation for [()] can be used instead.
//!
//! ### Weights and Benchmarking
//!
//! The pallet comes with pre-defined weights for its extrinsics that can be used during initial development
//! through the [pallet_governed_map::weights::SubstrateWeight] type.
//!
//! However, since data size limits and the on-change logic both can affect the weights, it is advisable to run
//! your own benchmark to account for their impact. See the documentation on `[crate::benchmarking]` for details.
//!
//! ### Configuring the pallet
//!
//! Once the above are defined, the pallet can finally be added to the runtime and configured like this:
//!
//! ```rust,ignore
//! impl pallet_governed_map::Config for Runtime {
//!     type MaxChanges = MaxChanges;
//!     type MaxKeyLength = MaxKeyLength;
//!     type MaxValueLength = MaxValueLength;
//!     type WeightInfo = pallet_governed_map::weights::SubstrateWeight<Runtime>;
//!
//!     type OnGovernedMappingChange = ChangeHandler;
//!
//!     #[cfg(feature = "runtime-benchmarks")]
//!     type BenchmarkHelper = ();
//! }
//! ```
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

pub use pallet::*;

use crate::alloc::string::{String, ToString};
use crate::weights::WeightInfo;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sidechain_domain::byte_string::*;
use sp_governed_map::*;

pub mod weights;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Current pallet version
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

		/// Handler called for each change in the governed mappings.
		///
		/// If your runtime does not need to react to any changes, a no-op implementation for [()] can be used.
		/// Otherwise, it is advised to benchmark the runtime and use your own weights to include weight consumed
		/// by the handler.
		type OnGovernedMappingChange: OnGovernedMappingChange<Self::MaxKeyLength, Self::MaxValueLength>;

		/// Weight functions for the pallet's extrinsics
		type WeightInfo: weights::WeightInfo;

		/// Helper functions required by the pallet's benchmarks to construct realistic input data.
		///
		/// An implementation for [()] is provided for simplicity. Chains that require more precise weights or
		/// expect an unusual number of parameter changes should implement this trait themselves in their runtime.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: crate::benchmarking::BenchmarkHelper<Self>;
	}

	/// Governed Map key type
	pub type MapKey<T> = BoundedString<<T as Config>::MaxKeyLength>;
	/// Governed Map value type
	pub type MapValue<T> = BoundedVec<u8, <T as Config>::MaxValueLength>;
	/// Governed Map change list
	pub type Changes<T> = BoundedVec<(MapKey<T>, Option<MapValue<T>>), <T as Config>::MaxChanges>;

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
		/// Phantom data marker
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
		fn decode_inherent_data(
			data: &InherentData,
		) -> Option<alloc::vec::Vec<GovernedMapChangeV1>> {
			data.get_data::<alloc::vec::Vec<GovernedMapChangeV1>>(&INHERENT_IDENTIFIER)
				.expect("Governed Map inherent data is not encoded correctly")
		}

		fn create_inherent_or_err(data: &InherentData) -> Result<Option<Call<T>>, InherentError> {
			use InherentError::*;

			let Some(raw_changes) = Self::decode_inherent_data(data) else { return Ok(None) };

			if raw_changes.len() > T::MaxChanges::get() as usize {
				return Err(TooManyChanges);
			}

			let mut changes = Changes::<T>::new();
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
		#[pallet::weight((T::WeightInfo::register_changes(changes.len() as u32), DispatchClass::Mandatory))]
		pub fn register_changes(origin: OriginFor<T>, changes: Changes<T>) -> DispatchResult {
			ensure_none(origin)?;

			log::info!("💾 Registering {} Governed Map changes", changes.len(),);

			for (key, value) in changes {
				let old_value = Mapping::<T>::get(&key);
				Mapping::<T>::set(&key, value.clone());
				T::OnGovernedMappingChange::on_governed_mapping_change(key, value, old_value);
			}

			Ok(())
		}

		/// Changes the address of the Governed Map validator used for observation.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		#[pallet::call_index(1)]
		#[pallet::weight((T::WeightInfo::set_main_chain_scripts(), DispatchClass::Normal))]
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

		/// Returns an iterator over all key-value pairs in the pallet storage, using unbound types.
		pub fn get_all_key_value_pairs_unbounded() -> impl Iterator<Item = (String, ByteString)> {
			Self::get_all_key_value_pairs()
				.map(|(key, value)| (key.to_string(), value.to_vec().into()))
		}

		/// Returns current pallet version.
		pub fn get_version() -> u32 {
			PALLET_VERSION
		}

		/// Returns the current main chain scripts
		pub fn get_main_chain_scripts() -> Option<MainChainScriptsV1> {
			MainChainScripts::<T>::get()
		}
	}
}
