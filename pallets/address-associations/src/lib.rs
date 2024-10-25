//! Pallet storing associations from main chain addresses to parter chain address.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::fmt::Debug;
	use sp_std::vec::Vec;

	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Main chain address type
		type MainChainAddress: Member + Parameter + MaxEncodedLen;

		/// Partner chain address type
		type PartnerChainAddress: Member + Parameter + MaxEncodedLen;

		/// Type of configuration used by the data source to identify association data on the main chain
		///
		/// This type is main chain-specific and should come from the observability implementation
		type ObservabilityConfigurationType: Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ Debug;
		/// Maximum number of new associations handled by the inherent in a single block
		///
		/// This number must be high enough for the chain to keep up with incoming data.
		type MaxNewAssociationsPerBlock: Get<u32>;
	}

	#[pallet::storage]
	pub type ObservabilityConfiguration<T: Config> =
		StorageValue<_, T::ObservabilityConfigurationType, OptionQuery>;

	#[pallet::storage]
	pub type AddressAssociations<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = T::MainChainAddress,
		Value = T::PartnerChainAddress,
		QueryKind = OptionQuery,
	>;

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Changes the observability configuration.
		///
		/// This extrinsic must be run either using `sudo` or some other chain governance mechanism.
		///
		/// Warning: this extrinsic also resets the internal state stored by the observability data source
		/// in the runtime and should be invoked only when necessary.
		#[pallet::call_index(1)]
		#[pallet::weight((1, DispatchClass::Normal))]
		pub fn set_main_chain_scripts(
			origin: OriginFor<T>,
			observability_config: T::ObservabilityConfigurationType,
		) -> DispatchResult {
			ensure_root(origin)?;
			ObservabilityConfiguration::<T>::put(observability_config.clone());
			log::info!("⚙️ Address association observability configuration changed to {observability_config:?}");
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the current pallet version.
		pub fn get_version() -> u32 {
			PALLET_VERSION
		}

		/// Retrieves all main chain - partner chain address associations from the runtime storage.
		pub fn get_all_address_associations() -> Vec<(T::MainChainAddress, T::PartnerChainAddress)>
		{
			AddressAssociations::<T>::iter().collect()
		}

		/// Retrieves the partner chain address for a given main chain address if the association for it exists.
		pub fn get_partner_chain_address_for(
			mc_addr: T::MainChainAddress,
		) -> Option<T::PartnerChainAddress> {
			AddressAssociations::<T>::get(mc_addr)
		}

		/// Retrieves the current observability configuration.
		///
		/// This configuration is only used by the observability data source providing association data.
		pub fn get_observability_configuration() -> Option<T::ObservabilityConfigurationType> {
			ObservabilityConfiguration::<T>::get()
		}
	}
}
