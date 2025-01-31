//! Pallet storing associations from main chain addresses to parter chain address.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

mod benchmarking;
pub mod weights;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use alloc::string::ToString;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::OriginFor;
	use sidechain_domain::{
		MainchainAddress, MainchainKeyHash, MainchainPublicKey, MainchainSignature, UtxoId,
	};
	// use frame_system::pallet_prelude::*;
	// use scale_info::prelude::fmt::Debug;
	use sp_address_associations::AddressAssociationSignedMessage;
	// use sp_std::vec::Vec;

	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type WeightInfo: crate::weights::WeightInfo;

		/// Partner chain address type
		type PartnerChainAddress: Member + Parameter + MaxEncodedLen;

		fn genesis_utxo() -> UtxoId;
	}

	#[pallet::storage]
	pub type AddressAssociations<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = MainchainAddress,
		Value = T::PartnerChainAddress,
		QueryKind = OptionQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		AddressAlreadyAssociated,
		InvalidMainchainAddress,
		InvalidMainchainPublicKey,
		InvalidMainchainSignature,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::associate_address())]
		pub fn associate_address(
			_origin: OriginFor<T>,
			mc_addr: MainchainAddress,
			pc_addr: T::PartnerChainAddress,
			signature: MainchainSignature,
			verification_key: MainchainPublicKey,
		) -> DispatchResult {
			let genesis_utxo = T::genesis_utxo();

			let mc_hash_1 = MainchainKeyHash::from_bech32_address(&mc_addr.to_string())
				.map_err(|_| Error::<T>::InvalidMainchainAddress)?;

			ensure!(
				!AddressAssociations::<T>::contains_key(&mc_addr),
				Error::<T>::AddressAlreadyAssociated
			);

			let address_association_message = AddressAssociationSignedMessage {
				mainchain_address: mc_addr.clone(),
				partnerchain_address: pc_addr.clone(),
				genesis_utxo,
			};

			let is_valid_signature =
				signature.verify(&verification_key, &address_association_message.encode());

			let mc_hash_2 = MainchainKeyHash::from_vkey(&verification_key.0);

			ensure!(mc_hash_1 == mc_hash_2, Error::<T>::InvalidMainchainPublicKey);

			ensure!(is_valid_signature, Error::<T>::InvalidMainchainSignature);

			AddressAssociations::<T>::insert(mc_addr, pc_addr);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the current pallet version.
		pub fn get_version() -> u32 {
			PALLET_VERSION
		}

		/// Retrieves all main chain - partner chain address associations from the runtime storage.
		pub fn get_all_address_associations(
		) -> impl Iterator<Item = (MainchainAddress, T::PartnerChainAddress)> {
			AddressAssociations::<T>::iter()
		}

		/// Retrieves the partner chain address for a given main chain address if the association for it exists.
		pub fn get_partner_chain_address_for(
			mc_addr: &MainchainAddress,
		) -> Option<T::PartnerChainAddress> {
			AddressAssociations::<T>::get(mc_addr)
		}
	}
}
