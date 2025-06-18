//! Pallet for storing Cardano configuration parameters in runtime storage.
//!
//! # Purpose of this pallet
//!
//! This pallet stores Cardano configuration parameters that are essential for Partner Chain operation,
//! including mainchain epoch configuration and consensus parameters. By storing these parameters in
//! runtime storage rather than relying on environment variables, we ensure all nodes have consistent
//! configuration as part of the chain state.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

pub use pallet::*;

use crate::weights::WeightInfo;
use core::marker::PhantomData;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use sidechain_domain::{
	cardano_config::CardanoConfig,
	mainchain_epoch::MainchainEpochConfig,
};
use sp_runtime::Perbill;
use sp_core::offchain::{Duration, Timestamp};

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
		/// Weight functions for the pallet's extrinsics
		type WeightInfo: weights::WeightInfo;

		/// Helper functions required by the pallet's benchmarks to construct realistic input data.
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: crate::benchmarking::BenchmarkHelper<Self>;
	}

	/// Error type used by this pallet's extrinsics
	#[pallet::error]
	pub enum Error<T> {
		/// Configuration has already been set and cannot be changed
		ConfigurationAlreadySet,
	}

	/// Stores the epoch duration in milliseconds
	#[pallet::storage]
	#[pallet::getter(fn epoch_duration_millis)]
	pub type EpochDurationMillis<T: Config> = StorageValue<_, u64, OptionQuery>;

	/// Stores the slot duration in milliseconds
	#[pallet::storage]
	#[pallet::getter(fn slot_duration_millis)]
	pub type SlotDurationMillis<T: Config> = StorageValue<_, u64, OptionQuery>;

	/// Stores the first epoch timestamp in milliseconds
	#[pallet::storage]
	#[pallet::getter(fn first_epoch_timestamp_millis)]
	pub type FirstEpochTimestampMillis<T: Config> = StorageValue<_, u64, OptionQuery>;

	/// Stores the first epoch number
	#[pallet::storage]
	#[pallet::getter(fn first_epoch_number)]
	pub type FirstEpochNumber<T: Config> = StorageValue<_, u32, OptionQuery>;

	/// Stores the first slot number
	#[pallet::storage]
	#[pallet::getter(fn first_slot_number)]
	pub type FirstSlotNumber<T: Config> = StorageValue<_, u64, OptionQuery>;

	/// Stores the Cardano security parameter (k)
	#[pallet::storage]
	#[pallet::getter(fn cardano_security_parameter)]
	pub type CardanoSecurityParameter<T: Config> = StorageValue<_, u32, OptionQuery>;

	/// Stores the Cardano active slots coefficient (f) as parts per billion
	#[pallet::storage]
	#[pallet::getter(fn cardano_active_slots_coeff)]
	pub type CardanoActiveSlotsCoeff<T: Config> = StorageValue<_, Perbill, OptionQuery>;

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// Initial Cardano configuration.
		pub cardano_config: Option<CardanoConfig>,
		/// Phantom data marker
		pub _marker: PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			if let Some(config) = &self.cardano_config {
				EpochDurationMillis::<T>::put(config.epoch_config.epoch_duration_millis.as_millis());
				SlotDurationMillis::<T>::put(config.epoch_config.slot_duration_millis.as_millis());
				FirstEpochTimestampMillis::<T>::put(config.epoch_config.first_epoch_timestamp_millis.as_unix_millis());
				FirstEpochNumber::<T>::put(config.epoch_config.first_epoch_number);
				FirstSlotNumber::<T>::put(config.epoch_config.first_slot_number);
				CardanoSecurityParameter::<T>::put(config.cardano_security_parameter);
				CardanoActiveSlotsCoeff::<T>::put(Perbill::from_float(config.cardano_active_slots_coeff as f64));
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the Cardano configuration. This can only be called once.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_cardano_config())]
		pub fn set_cardano_config(
			origin: OriginFor<T>,
			config: CardanoConfig,
		) -> DispatchResult {
			ensure_root(origin)?;
			
			ensure!(
				!Self::is_configured(),
				Error::<T>::ConfigurationAlreadySet
			);

			EpochDurationMillis::<T>::put(config.epoch_config.epoch_duration_millis.0);
			SlotDurationMillis::<T>::put(config.epoch_config.slot_duration_millis.0);
			FirstEpochTimestampMillis::<T>::put(config.epoch_config.first_epoch_timestamp_millis.0);
			FirstEpochNumber::<T>::put(config.epoch_config.first_epoch_number);
			FirstSlotNumber::<T>::put(config.epoch_config.first_slot_number);
			CardanoSecurityParameter::<T>::put(config.cardano_security_parameter);
			CardanoActiveSlotsCoeff::<T>::put(Perbill::from_float(config.cardano_active_slots_coeff));

			log::info!("🔧 Cardano configuration set");
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the complete Cardano configuration
		pub fn get_cardano_config() -> Option<CardanoConfig> {
			let epoch_config = Self::get_mainchain_epoch_config()?;
			let security_parameter = CardanoSecurityParameter::<T>::get()?;
			let active_slots_coeff = CardanoActiveSlotsCoeff::<T>::get()?;

			Some(CardanoConfig {
				epoch_config,
				cardano_security_parameter: security_parameter,
				cardano_active_slots_coeff: active_slots_coeff.deconstruct() as f32 / 1_000_000_000.0,
			})
		}

		/// Returns the mainchain epoch configuration
		pub fn get_mainchain_epoch_config() -> Option<MainchainEpochConfig> {
			let epoch_duration_millis = EpochDurationMillis::<T>::get()?;
			let slot_duration_millis = SlotDurationMillis::<T>::get()?;
			let first_epoch_timestamp_millis = FirstEpochTimestampMillis::<T>::get()?;
			let first_epoch_number = FirstEpochNumber::<T>::get()?;
			let first_slot_number = FirstSlotNumber::<T>::get()?;

			Some(MainchainEpochConfig {
				epoch_duration_millis: Duration::from_millis(epoch_duration_millis),
				slot_duration_millis: Duration::from_millis(slot_duration_millis),
				first_epoch_timestamp_millis: Timestamp::from_unix_millis(first_epoch_timestamp_millis),
				first_epoch_number,
				first_slot_number,
			})
		}

		/// Returns the Cardano security parameter (k)
		pub fn get_cardano_security_parameter() -> Option<u32> {
			CardanoSecurityParameter::<T>::get()
		}

		/// Returns the Cardano active slots coefficient (f)
		pub fn get_cardano_active_slots_coeff() -> Option<f32> {
			CardanoActiveSlotsCoeff::<T>::get()
				.map(|perbill| perbill.deconstruct() as f32 / 1_000_000_000.0)
		}

		/// Returns whether the configuration has been set
		pub fn is_configured() -> bool {
			EpochDurationMillis::<T>::exists() &&
			SlotDurationMillis::<T>::exists() &&
			FirstEpochTimestampMillis::<T>::exists() &&
			FirstEpochNumber::<T>::exists() &&
			FirstSlotNumber::<T>::exists() &&
			CardanoSecurityParameter::<T>::exists() &&
			CardanoActiveSlotsCoeff::<T>::exists()
		}

		/// Returns current pallet version
		pub fn get_version() -> u32 {
			PALLET_VERSION
		}
	}
}
