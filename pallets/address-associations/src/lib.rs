//! Pallet storing associations from main chain addresses to parter chain address.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use scale_info::prelude::fmt::Debug;
	use sp_address_associations::*;
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

		/// Type representing internal state saved by the observability layer in the storage for its operation
		type SyncStateType: Member + Parameter + MaybeSerializeDeserialize + MaxEncodedLen + Debug;

		/// Maximum number of new associations handled by the inherent in a single block
		///
		/// This number must be high enough for the chain to keep up with incoming data.
		type MaxNewAssociationsPerBlock: Get<u32>;
	}

	#[pallet::storage]
	pub type ObservabilityConfiguration<T: Config> =
		StorageValue<_, T::ObservabilityConfigurationType, OptionQuery>;

	#[pallet::storage]
	pub type SyncState<T: Config> = StorageValue<_, T::SyncStateType, OptionQuery>;

	#[pallet::storage]
	pub type AddressAssociations<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = T::MainChainAddress,
		Value = T::PartnerChainAddress,
		QueryKind = OptionQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// Initial observability configuration
		pub observability_config: Option<T::ObservabilityConfigurationType>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { observability_config: None }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			ObservabilityConfiguration::<T>::set(self.observability_config.clone());
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Adds new address associations to the pallet storage.
		///
		/// This extrinsic should only be invoked as an inherent.
		#[pallet::call_index(0)]
		#[pallet::weight((0, DispatchClass::Mandatory))]
		pub fn add(
			origin: OriginFor<T>,
			associations: BoundedVec<
				(T::MainChainAddress, T::PartnerChainAddress),
				T::MaxNewAssociationsPerBlock,
			>,
			new_sync_state: T::SyncStateType,
		) -> DispatchResult {
			ensure_none(origin)?;
			assert!(
				ObservabilityConfiguration::<T>::exists(),
				"BUG: Address associations inherent should not be run unless the main chain scripts are set."
			);
			for (mc, pc) in associations.iter() {
				assert!(
					!AddressAssociations::<T>::contains_key(mc),
					"Attempt to overwrite an existing address association."
				);
				AddressAssociations::<T>::insert(mc.clone(), pc.clone());
			}
			SyncState::<T>::put(new_sync_state.clone());
			log::info!(
				"🔗 {} new address associations added. New sync state is {:?}.",
				associations.len(),
				new_sync_state
			);
			Ok(())
		}

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
			SyncState::<T>::set(None);
			log::info!("⚙️ Address association observability configuration changed to {observability_config:?}");
			Ok(())
		}
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = sp_address_associations::InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier =
			sp_address_associations::INHERENT_IDENTIFIER;

		/// Create address association inherent, if new associations were observed
		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let AddressAssociationInherentData { associations, new_sync_state } =
				Self::decode_inherent_data(data)?;
			assert!(
				associations.len() as u32 <= T::MaxNewAssociationsPerBlock::get(),
				"New address associations in inherent data must be less then the maximum."
			);
			let associations = Self::filter_out_overwrite_attempts(associations);
			let associations = BoundedVec::truncate_from(associations);
			if associations.is_empty() {
				None
			} else {
				Some(Self::Call::add { associations, new_sync_state })
			}
		}

		fn check_inherent(call: &Self::Call, data: &InherentData) -> Result<(), Self::Error> {
			let Self::Call::add { associations, new_sync_state } = call else {
				return Ok(());
			};

			let AddressAssociationInherentData {
				associations: expected_associations,
				new_sync_state: expected_sync_state,
			} = Self::decode_inherent_data(data).unwrap();
			let expected_associations = Self::filter_out_overwrite_attempts(expected_associations);

			if associations.to_vec() != expected_associations {
				return Err(Self::Error::IncorrectAssociations);
			}
			if *new_sync_state != expected_sync_state {
				return Err(Self::Error::IncorrectNewSyncState);
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::add { .. })
		}

		fn is_inherent_required(data: &InherentData) -> Result<Option<Self::Error>, Self::Error> {
			match Self::decode_inherent_data(data) {
				None => Ok(None),
				Some(AddressAssociationInherentData { associations, .. }) => {
					if !Self::filter_out_overwrite_attempts(associations).is_empty() {
						Ok(Some(Self::Error::InherentRequired))
					} else {
						Ok(None)
					}
				},
			}
		}
	}
	impl<T: Config> Pallet<T> {
		fn decode_inherent_data(
			inherent_data: &InherentData,
		) -> Option<
			AddressAssociationInherentData<
				T::MainChainAddress,
				T::PartnerChainAddress,
				T::SyncStateType,
			>,
		> {
			AddressAssociationInherentData::from_inherent_data(inherent_data)
				.expect("Failed to decode address association inherent data.")
		}
		fn filter_out_overwrite_attempts(
			associations: Vec<(T::MainChainAddress, T::PartnerChainAddress)>,
		) -> Vec<(T::MainChainAddress, T::PartnerChainAddress)> {
			(associations.into_iter())
				.filter(|(key, _)| !AddressAssociations::<T>::contains_key(key))
				.collect()
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

		/// Retrieves the currently saves sync state.
		///
		/// This state is only used by the observability data source providing association data.
		pub fn get_current_sync_state() -> Option<T::SyncStateType> {
			SyncState::<T>::get()
		}
	}
}
