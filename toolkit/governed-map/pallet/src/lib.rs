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
//!
//! #### `MaxChanges`
//! The maximum number of changes that will be processed in one inherent invocation. Changes
//! are understood as the diff between previously stored mappings and the currently observed
//! ones, as opposed to raw on-chain events altering those mappings.
//!
//! **Important**: This number must be high enough to guarantee the inherent will always have
//! the capacity required to process incoming changes. If the number of changes exceeds this
//! limit, [TooManyChanges][InherentError::TooManyChanges] error will be raised stalling block production.
//!
//! If this error occurs on a live chain, then the only way of fixing it is to change the
//! mappings on Cardano close enough to the last state registered in the pallet to bring the
//! change count below the limit.
//! Setting this limit above the expected number of keys in use is a safe option.
//!
//! #### `MaxKeyLength`
//! maximum length of keys that can be used.
//!
//! Important: If a key is set in the Governed Map that exceeds this lenght limit, the
//! [KeyExceedsBounds][InherentError::KeyExceedsBounds] error will be raised stalling block production, with
//! the only recovery path being removal of this key so it is no longer in the change set.
//!
//! #### `MaxValueLength`
//! Maximum length of the value under a key. Same considerations as for `MaxKeyLength` apply.
//!
//! #### Defining the values in the code
//!
//! Once the limit values are decided, define them in your runtime, like so:
//! ```rust
//! frame_support::parameter_types! {
//!        pub const MaxChanges: u32 = 16;
//!        pub const MaxKeyLength: u32 = 64;
//!        pub const MaxValueLength: u32 = 512;
//! }
//! ```
//!
//! If at any point a need arises to either support higher volume of parameter changes or increase the maximum
//! length of keys and values in the mappings, it can be achieved through a runtime upgrade that modifies the
//! pallet's configuration.
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
//! through the [SubstrateWeight][crate::weights::SubstrateWeight] type.
//!
//! However, since data size limits and the on-change logic both can affect the weights, it is advisable to run
//! your own benchmark to account for their impact. See the documentation on [crate::benchmarking] for details.
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
//!
//! ### Setting the main chain scripts
//!
//! For the data sources to be able to observe the Governed Map state on Cardano, the pallet stores and exposes
//! relevant addresses and script hashes which are necessary to query main chain state. Their values need to be
//! set before the feature can be fully functional. How this is done depends on whether the pallet is present from
//! the genesis block or added later to a live chain.
//!
//! #### Configuring the addresses at genesis
//!
//! If the pallet is included in the runtime from genesis block, the scripts can be configured in the genesis config
//! of your runtime:
//! ```rust
//! # use sidechain_domain::*;
//! # use std::str::FromStr;
//! # fn build_genesis<T: pallet_governed_map::Config>() -> pallet_governed_map::GenesisConfig<T> {
//! pallet_governed_map::GenesisConfig {
//!     main_chain_scripts: Some(pallet_governed_map::MainChainScriptsV1 {
//!         asset_policy_id: PolicyId::from_hex_unsafe("00000000000000000000000000000000000000000000000000000001"),
//!         validator_address: MainchainAddress::from_str("test_addr1").unwrap(),
//!     }),
//!     ..Default::default()
//! }
//! # }
//! ```
//! At the same time `main_chain_scripts` field is optional and can be set to [None] if you wish to postpone setting
//! the scripts for whatever reason.
//!
//! #### Setting the addresses via extrinsic
//!
//! If the pallet is added to a running chain, it will initailly have no main chain scripts set and remain inactive
//! until they are set. See section "Updating main chain scripts" for more information.
//!
//! ## Updating main chain scripts
//!
//! To allow the Partner Chain's governance to set and update main chain script values, the pallet provides the
//! [set_main_chain_scripts][Call::set_main_chain_scripts] extrinsic which updates the script values in its storage.
//! This extrinsic is required to be run with root access either via the `sudo` pallet or other governance mechanism.
//!
//! Every time [set_main_chain_scripts][Call::set_main_chain_scripts] is successfuly invoked, the pallet will update
//! its tracked Governed Map state to be congruent with the mappings pointed to by the updates scripts on the next
//! Partner Chain block.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

pub use pallet::*;
pub use sp_governed_map::MainChainScriptsV1;

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
		/// This value *must* be high enough for all changes to be registered in one block.
		/// Setting this to a value higher than the total number of parameters in the Governed Map guarantees that.
		#[pallet::constant]
		type MaxChanges: Get<u32>;

		/// Maximum length of the key in the Governed Map in bytes.
		///
		/// This value *must* be high enough not to be exceeded by any key stored on Cardano.
		#[pallet::constant]
		type MaxKeyLength: Get<u32>;

		/// Maximum length of data stored under a single key in the Governed Map
		///
		/// This value *must* be high enough not to be exceeded by any value stored on Cardano.
		#[pallet::constant]
		type MaxValueLength: Get<u32>;

		/// Handler called for each change in the governed mappings.
		///
		/// If your runtime does not need to react to any changes, a no-op implementation for [()] can be used.
		/// Otherwise, it is advised to benchmark the runtime and use your own weights to include weight consumed
		/// by the handler.
		type OnGovernedMappingChange: OnGovernedMappingChange<Self::MaxKeyLength, Self::MaxValueLength>;

		/// Origin for governance calls
		type MainChainScriptsOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Weight functions for the pallet's extrinsics
		type WeightInfo: weights::WeightInfo;

		/// Helper functions required by the pallet's benchmarks to construct realistic input data.
		///
		/// An implementation for [()] is provided for simplicity. Chains that require more precise weights or
		/// expect an unusual number of parameter changes should implement this trait themselves in their runtime.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: crate::benchmarking::BenchmarkHelper<Self>;
	}

	/// Error type used  by this pallet's extrinsics
	#[pallet::error]
	pub enum Error<T> {
		/// Signals that the inherent has been called again in the same block
		InherentCalledTwice,
		/// MainChainScript is not set, registration of changes is not allowed
		MainChainScriptNotSet,
	}

	/// Governed Map key type
	pub type MapKey<T> = BoundedString<<T as Config>::MaxKeyLength>;
	/// Governed Map value type
	pub type MapValue<T> = BoundedVec<u8, <T as Config>::MaxValueLength>;
	/// Governed Map change list
	pub type Changes<T> =
		BoundedBTreeMap<MapKey<T>, Option<MapValue<T>>, <T as Config>::MaxChanges>;

	/// Stores the initialization state of the pallet
	///
	/// The pallet is considered uninitialized if no inherent was executed since the genesis block or
	/// since the last change of the main chain scripts.
	#[pallet::storage]
	pub type Initialized<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Stores the block number of the last time mapping changes were registered
	#[pallet::storage]
	pub type LastUpdateBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

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
		pub main_chain_scripts: Option<MainChainScriptsV1>,
		/// Phantom data marker
		pub _marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MainChainScripts::<T>::set(self.main_chain_scripts.clone());
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
				Some(changes) if changes.is_empty() && Initialized::<T>::get() => Ok(None),
				Some(_) => Ok(Some(Self::Error::InherentMissing)),
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn decode_inherent_data(data: &InherentData) -> Option<GovernedMapInherentDataV1> {
			data.get_data::<GovernedMapInherentDataV1>(&INHERENT_IDENTIFIER)
				.expect("Governed Map inherent data is not encoded correctly")
		}

		fn create_inherent_or_err(data: &InherentData) -> Result<Option<Call<T>>, InherentError> {
			use InherentError::*;

			let Some(raw_changes) = Self::decode_inherent_data(data) else { return Ok(None) };

			if raw_changes.is_empty() && Initialized::<T>::get() {
				return Ok(None);
			}

			if raw_changes.len() > T::MaxChanges::get() as usize {
				return Err(TooManyChanges);
			}

			let mut changes = Changes::<T>::new();
			for (key, new_value) in raw_changes {
				let new_value = match new_value {
					Some(new_value) => Some(Self::bound_value(&key, new_value)?),
					None => None,
				};
				let key = Self::bound_key(&key)?;
				changes.try_insert(key, new_value).expect("Number of changes is below maximum");
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
			let current_block = frame_system::Pallet::<T>::block_number();
			ensure!(
				LastUpdateBlock::<T>::get().map_or(true, |last_block| last_block < current_block),
				Error::<T>::InherentCalledTwice
			);
			LastUpdateBlock::<T>::put(current_block);

			// ensure!(MainChainScripts::<T>::exists(), Error::<T>::MainChainScriptNotSet);

			if Initialized::<T>::get() {
				log::info!("💾 Registering {} Governed Map changes", changes.len(),);
			} else {
				log::info!(
					"💾 Reinitializing the Governed Map pallet with {} changes",
					changes.len(),
				);
				Initialized::<T>::set(true);
			}

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
			T::MainChainScriptsOrigin::ensure_origin(origin)?;
			MainChainScripts::<T>::put(new_main_chain_script.clone());
			Initialized::<T>::set(false);
			log::info!("🗂️ Governed Map main chain scripts updated to {new_main_chain_script:?}");
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the value under `key` or [None] otherwise.
		pub fn get_key_value(key: &MapKey<T>) -> Option<BoundedVec<u8, T::MaxValueLength>> {
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

		/// Returns initialization status of the pallet
		pub fn is_initialized() -> bool {
			Initialized::<T>::get()
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
