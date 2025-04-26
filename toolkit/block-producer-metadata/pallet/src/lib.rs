//! Pallet storing metadata for Partner Chain block producers.
//!
//! ## Purpose of this pallet
//!
//! This pallet enables Partner Chain block producers to provide information about themselves that can
//! then be displayed by chain explorers and other tools to potential delegators looking for pools to
//! delegate to.
//!
//! ## Usage - PC Builders
//!
//! PC Builders wishing to include this pallet in their runtime should first define a `BlockProducerMetadata`
//! type for their runtime to use, eg.
//!
//! ```rust
//! use sidechain_domain::byte_string::BoundedString;
//! use sp_core::{Encode, ConstU32, Decode, MaxEncodedLen};
//!
//! type MaxNameLength = ConstU32<64>;
//! type MaxDescriptionLength = ConstU32<512>;
//! type MaxUrlLength = ConstU32<256>;
//!
//! #[derive(Encode, Decode, MaxEncodedLen)]
//! pub struct BlockProducerMetadata {
//!     pub name: BoundedString<MaxNameLength>,
//!     pub description: BoundedString<MaxDescriptionLength>,
//!     pub logo_url: BoundedString<MaxUrlLength>
//! }
//! ```
//!
//! This type can be arbitrary to allow PC Builders to include any data that would be relevant to their chain.
//! However, care should be taken to keep its size to minimum to avoid inflating on-chain storage size by eg.
//! linking to off-chain storage for bulkier data:
//! ```
//! # use sidechain_domain::byte_string::*;
//! # use sp_core::ConstU32;
//! # type MaxUrlLength = ConstU32<256>;
//! pub struct BlockProducerMetadataType {
//!     pub url: BoundedString<MaxUrlLength>,
//!     pub hash: SizedByteString<32>,
//! }
//! ```
//!
//! Once the metadata type is defined, the pallet can be added to the runtime and should be configured:
//! ```rust,ignore
//! impl pallet_block_producer_metadata::Config for Runtime {
//!     type WeightInfo = pallet_block_producer_metadata::weights::SubstrateWeight<Runtime>;
//!
//!     type BlockProducerMetadata = BlockProducerMetadata;
//!
//!     fn genesis_utxo() -> sidechain_domain::UtxoId {
//!         Sidechain::genesis_utxo()
//!     }
//! }
//! ```
//!
//! Here, besides providing the metadata type and using weights already provided with the pallet, we are also
//! wiring the `genesis_utxo` function to fetch the chain's genesis UTXO from the `pallet_sidechain` pallet.
//!
//! At this point, the pallet is ready to be used.
//!
//! ### Signing command
//!
//! To ensure that only the block producer is able to update their own metadata, a signature is required by the
//! pallet's extrinsic. To make it easy for PC Builders to provide their users with a signing utility, the
//! `cli_commands` crate includes a command for signing the appropriate message. Consult the crate's own
//! documentation for more details.
//!
//! ### Benchmarking
//!
//! See documentation of [benchmarking] module.
//!
//! ### RPC
//!
//! See documentation of `pallet_block_producer_metadata_rpc` crate.
//!
//! ## Usage - PC Users
//!
//! This pallet exposes a single extrinsic `upsert_metadata` for current or prospective block producers to add or
//! update their metadata. The extrinsic requires a valid signature, which the user should prepare using the
//! `sign-block-producer-metadata` command provided by the chain's node. This command returns the signature
//! and the metadata encoded as hex bytes.
//!
//! After the signature has been obtained, the user should submit the `upsert_metadata` extrinsic (eg. using PolkadotJS)
//! providing:
//! - *metadata value*: when using PolkadotJS UI, care must be taken to submit the same values that were passed to the CLI
//! - *signature* returned by the CLI
//! - *cross-chain public key* corresponding to the private key used for signing with the CLI

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

pub use pallet::*;

pub mod benchmarking;
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

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

	/// Current version of the pallet
	pub const PALLET_VERSION: u32 = 1;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Weight information for this pallet's extrinsics
		type WeightInfo: crate::weights::WeightInfo;

		/// Block producer metadata type
		type BlockProducerMetadata: Member + Parameter + MaxEncodedLen;

		/// Should return the chain's genesis UTXO
		fn genesis_utxo() -> UtxoId;

		/// Helper providing mock values for use in benchmarks
		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: benchmarking::BenchmarkHelper<Self::BlockProducerMetadata>;
	}

	/// Storage mapping from block producers to their metadata
	#[pallet::storage]
	pub type BlockProducerMetadataStorage<T: Config> = StorageMap<
		Hasher = Blake2_128Concat,
		Key = CrossChainKeyHash,
		Value = T::BlockProducerMetadata,
		QueryKind = OptionQuery,
	>;

	/// Error type returned by this pallet's extrinsic
	#[pallet::error]
	pub enum Error<T> {
		/// Signals that the signature submitted to `upsert_metadata` does not match the metadata and public key
		InvalidMainchainSignature,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Inserts or updates metadata for the block producer identified by `cross_chain_pub_key`.
		///
		/// Arguments:
		/// - `metadata`: new metadata value
		/// - `signature`: a signature of [MetadataSignedMessage] created from this inherent's arguments
		///   and the current Partner Chain's genesis UTXO, created using the private key corresponding
		///   to `cross_chain_pub_key`
		/// - `cross_chain_pub_key`: public key identifying the block producer
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
