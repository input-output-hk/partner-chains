//! Pallet storing associations from main chain public key to parter chain address.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

mod benchmarking;
pub mod weights;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

use parity_scale_codec::Encode;
use sidechain_domain::{MainchainPublicKey, UtxoId};

#[derive(Debug, Clone, Encode)]
pub struct AddressAssociationSignedMessage<PartnerChainAddress> {
	pub mainchain_public_key: MainchainPublicKey,
	pub partnerchain_address: PartnerChainAddress,
	pub genesis_utxo: UtxoId,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::OriginFor;
	use sidechain_domain::{
		MainchainKeyHash, MainchainPublicKey, MainchainSignature, UtxoId, MAINCHAIN_SIGNATURE_LEN,
	};

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
		Key = MainchainKeyHash,
		Value = T::PartnerChainAddress,
		QueryKind = OptionQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		MainchainKeyAlreadyAssociated,
		InvalidMainchainSignature,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::associate_address())]
		pub fn associate_address(
			_origin: OriginFor<T>,
			partnerchain_address: T::PartnerChainAddress,
			signature: [u8; MAINCHAIN_SIGNATURE_LEN],
			mainchain_public_key: MainchainPublicKey,
		) -> DispatchResult {
			let signature: MainchainSignature = signature.into();

			let genesis_utxo = T::genesis_utxo();

			let mc_vkey_hash = MainchainKeyHash::from_vkey(&mainchain_public_key.0);

			ensure!(
				!AddressAssociations::<T>::contains_key(&mc_vkey_hash),
				Error::<T>::MainchainKeyAlreadyAssociated
			);

			let address_association_message = AddressAssociationSignedMessage {
				mainchain_public_key: mainchain_public_key.clone(),
				partnerchain_address: partnerchain_address.clone(),
				genesis_utxo,
			};

			let is_valid_signature =
				signature.verify(&mainchain_public_key, &address_association_message.encode());

			ensure!(is_valid_signature, Error::<T>::InvalidMainchainSignature);

			AddressAssociations::<T>::insert(mc_vkey_hash, partnerchain_address);
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
		) -> impl Iterator<Item = (MainchainKeyHash, T::PartnerChainAddress)> {
			AddressAssociations::<T>::iter()
		}

		/// Retrieves the partner chain address for a given main chain public key if the association for it exists.
		pub fn get_partner_chain_address_for(
			mc_addr: &MainchainPublicKey,
		) -> Option<T::PartnerChainAddress> {
			AddressAssociations::<T>::get(MainchainKeyHash::from_vkey(&mc_addr.0))
		}
	}
}
