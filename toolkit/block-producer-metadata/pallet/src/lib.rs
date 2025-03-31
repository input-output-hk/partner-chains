//! Pallet storing block producer metadata for SPO public key hashes.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

pub mod benchmarking;
pub mod weights;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod tests;

use parity_scale_codec::Encode;
use sidechain_domain::{CrossChainKeyHash, CrossChainPublicKey};
use sp_block_producer_metadata::MetadataSignedMessage;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::weights::WeightInfo;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::OriginFor;
	use sidechain_domain::{CrossChainSignature, UtxoId};

	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type WeightInfo: crate::weights::WeightInfo;

		/// Metadata type
		type BlockProducerMetadata: Member + Parameter + MaxEncodedLen;

		fn genesis_utxo() -> UtxoId;

		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: benchmarking::BenchmarkHelper<Self::BlockProducerMetadata>;
	}

	#[pallet::storage]
	pub type BlockProducerMetadataStorage<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = CrossChainKeyHash,
		Value = T::BlockProducerMetadata,
		QueryKind = OptionQuery,
	>;

	#[pallet::error]
	pub enum Error<T> {
		InvalidMainchainSignature,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::upsert_metadata())]
		pub fn upsert_metadata(
			_origin: OriginFor<T>,
			metadata: T::BlockProducerMetadata,
			signature: CrossChainSignature,
			cross_chain_pub_key: CrossChainPublicKey,
		) -> DispatchResult {
			let genesis_utxo = T::genesis_utxo();

			let cross_chain_key_hash = cross_chain_pub_key.hash();

			let metadata_message = MetadataSignedMessage {
				cross_chain_pub_key: cross_chain_pub_key.clone(),
				metadata: metadata.clone(),
				genesis_utxo,
			};

			let is_valid_signature =
				signature.verify(&cross_chain_pub_key, &metadata_message.encode()).is_ok();

			ensure!(is_valid_signature, Error::<T>::InvalidMainchainSignature);

			BlockProducerMetadataStorage::<T>::insert(cross_chain_key_hash, metadata);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns the current pallet version.
		pub fn get_version() -> u32 {
			PALLET_VERSION
		}

		/// Retrieves the metadata for a given SPO public key if it exists.
		pub fn get_metadata_for(
			cross_chain_pub_key: &CrossChainPublicKey,
		) -> Option<T::BlockProducerMetadata> {
			BlockProducerMetadataStorage::<T>::get(cross_chain_pub_key.hash())
		}
	}
}
