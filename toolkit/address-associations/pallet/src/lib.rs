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
use sidechain_domain::MainchainKeyHash;
use sidechain_domain::{StakePublicKey, UtxoId};

#[derive(Debug, Clone, Encode)]
pub struct AddressAssociationSignedMessage<PartnerChainAddress> {
	pub stake_public_key: StakePublicKey,
	pub partnerchain_address: PartnerChainAddress,
	pub genesis_utxo: UtxoId,
}

pub trait OnNewAssociation<PartnerChainAddress> {
	fn on_new_association(
		partner_chain_address: PartnerChainAddress,
		main_chain_key_hash: MainchainKeyHash,
	);
}

impl<PartnerChainAddress> OnNewAssociation<PartnerChainAddress> for () {
	fn on_new_association(
		_partner_chain_address: PartnerChainAddress,
		_main_chain_key_hash: MainchainKeyHash,
	) {
	}
}
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::OriginFor;
	use sidechain_domain::{MainchainKeyHash, StakeKeySignature, StakePublicKey, UtxoId};

	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type WeightInfo: crate::weights::WeightInfo;

		/// Partner chain address type
		type PartnerChainAddress: Member + Parameter + MaxEncodedLen;

		fn genesis_utxo() -> UtxoId;

		type OnNewAssociation: OnNewAssociation<Self::PartnerChainAddress>;
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
			signature: StakeKeySignature,
			stake_public_key: StakePublicKey,
		) -> DispatchResult {
			let genesis_utxo = T::genesis_utxo();

			let stake_key_hash = stake_public_key.hash();

			ensure!(
				!AddressAssociations::<T>::contains_key(&stake_key_hash),
				Error::<T>::MainchainKeyAlreadyAssociated
			);

			let address_association_message = AddressAssociationSignedMessage {
				stake_public_key: stake_public_key.clone(),
				partnerchain_address: partnerchain_address.clone(),
				genesis_utxo,
			};

			let is_valid_signature =
				signature.verify(&stake_public_key, &address_association_message.encode());

			ensure!(is_valid_signature, Error::<T>::InvalidMainchainSignature);

			AddressAssociations::<T>::insert(stake_key_hash.clone(), partnerchain_address.clone());

			T::OnNewAssociation::on_new_association(partnerchain_address, stake_key_hash);

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
			stake_public_key: &StakePublicKey,
		) -> Option<T::PartnerChainAddress> {
			AddressAssociations::<T>::get(stake_public_key.hash())
		}
	}
}
