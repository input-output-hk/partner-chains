//! Pallet storing associations from main chain public key to parter chain address.
//!
//! ## Purpose of this pallet
//!
//! This pallet establishes a many-to-one mapping from Cardano staking keys to Partner Chain addresses.
//! The purpose of this mapping is primarily to indicate the local PC address to be the recipient of any
//! block production rewards or cross-chain token transfers credited to a Cardano key holders on a Partner
//! Chain. Some intended scenarios inlude:
//! 1. ADA delegators become eligible for block rewards due to their stake poool's operator participating
//!    in a Partner Chain network. The on-chain payout mechanism uses data provided by this pallet to
//!    identify each delegator's Partner Chain address based on their Cardano staking key.
//! 2. A Partner Chain develops its own cross-chain bridge from Cardano. A Cardano user associates their
//!    Cardano public key with a Partner Chain address that they control. The user then uses the bridge
//!    to send some tokens to themselves. The receiving logic in the Partner Chain ledger then uses this
//!    pallet's data to identify the user's PC account and comlete the transfer.
//!
//! ## Usage - PC Builder
//!
//! This pallet is self-contained and doesn't need any node support.
//! To include the pallet in the runtime, one must only provide its configuration.
//! Consult documentation of [pallet::Config] for explanation of all configuration fields.
//!
//! For the pallet to verify the validity of address associations submitted by Users, it requires a signature
//! created using the Cardano private key corresponding to the public key being associated. A dedicated
//! CLI command for creating the signature is provided by `cli_commands` crate. Consult the crate's
//! documentation for more information.
//!
//! ## Usage - PC User
//!
//! This pallet expoes a single extrinsic `associate_address` accepting the Cardano public key
//! and Partner Chain address to be associated together with a signature confirming that the submitter is
//! the owner of the associated Cardano public key.
//!
//! To obtain the signature, the User should use the dedicated signing command wired into the Partner Chain
//! node executable.
//!
//! *Important*: For compliance reasons, all address associations are final and can not be changed.
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

pub use pallet::*;

mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use parity_scale_codec::Encode;
use sidechain_domain::MainchainKeyHash;
use sidechain_domain::{StakePublicKey, UtxoId};

/// Schema of the message signed by a User to verify validity of submitted address association
#[derive(Debug, Clone, Encode)]
pub struct AddressAssociationSignedMessage<PartnerChainAddress> {
	/// Cardano stake public key to be associated
	pub stake_public_key: StakePublicKey,
	/// Partner Chain address to be associated
	pub partnerchain_address: PartnerChainAddress,
	/// Genesis UTXO of the Partner Chain on which the association is to be registered
	pub genesis_utxo: UtxoId,
}

/// Handler for new associations
pub trait OnNewAssociation<PartnerChainAddress> {
	/// Function called every time a new address association is created
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

	/// Current version of the pallet
	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Weight information on extrinsic in the pallet. For convenience weights in [weights] module can be used.
		type WeightInfo: crate::weights::WeightInfo;

		/// Type representing a local PC address. This can be a standard Substrate address, an
		/// account ID, or some address type specific to the Partner Chain.
		type PartnerChainAddress: Member + Parameter + MaxEncodedLen;

		/// Function returning the genesis UTXO of the Partner Chain.
		/// This typically should be wired with the `genesis_utxo` function exposed by `pallet_sidechain`.
		fn genesis_utxo() -> UtxoId;

		/// Handler that is called for each new address association.
		///
		/// If no handling logic is needed, [()] can be used for a no-op implementation.
		type OnNewAssociation: OnNewAssociation<Self::PartnerChainAddress>;
	}

	/// Storage of address association
	#[pallet::storage]
	pub type AddressAssociations<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = MainchainKeyHash,
		Value = T::PartnerChainAddress,
		QueryKind = OptionQuery,
	>;

	/// Error type returned by the pallet's extrinsic
	#[pallet::error]
	pub enum Error<T> {
		/// Signals that the Cardano key is already associated
		MainchainKeyAlreadyAssociated,
		/// Signals an invalid Cardano key signature
		InvalidMainchainSignature,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Extrinsic creating a new address association.
		///
		/// `signature` is expected to be a signature of the Cardano private key corresponding to `stake_public_key`
		/// of [AddressAssociationSignedMessage] created using the associated public key, address and the genesis UTXO
		/// of the particular Partner Chain it is being submitted to.
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

			AddressAssociations::<T>::insert(stake_key_hash, partnerchain_address.clone());

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
		pub fn get_all_address_associations()
		-> impl Iterator<Item = (MainchainKeyHash, T::PartnerChainAddress)> {
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
