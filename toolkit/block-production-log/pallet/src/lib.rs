//! A Substrate pallet maintaining a consumable log of block production information.
//!
//! ## Purpose of this pallet
//!
//! This pallet keeps a log containing block producer IDs along with times of blocks produced by them.
//! This log is updated every block and is meant to consumed by other features.
//! The intended use of this pallet within the Partner Chains SDK is to expose block production data for consumption
//! by the Block Participation feature implemented by the `sp_block_participation` and `pallet_block_participation`
//! crates.
//!
//! ## Usage - PC Builder
//!
//! ### Adding to the runtime
//!
//! The feature requires two types to be defind by the chain builders in their code:
//! - `BlockProducerId`: the type representing the block author
//! - `Moment`: a moment in time when the block was produced, which carries enough information to
//!             calculate the block's author. Typcally, this type can be a timestamp, or a slot,
//!             depending on the consensus mechanism used, but can be a richer type if needed.
//!
//! In addition, implementations of [GetAuthor] and [GetMoment] must be provided that can be used to
//! retrieve the current block's author and moment when it was produced.
//!
//! An example configuration for a runtime using Aura consensus and Partner Chain toolkit's session management
//! pallet might look like this:
//!
//! ```rust,ignore
//! impl pallet_block_production_log::Config for Runtime {
//!     type BlockProducerId = BlockAuthor;
//!
//!     type Moment = Slot;
//!
//!     type GetMoment = FromStorage<pallet_aura::CurrentSlot<Runtime>>;
//!     type GetAuthor = FromFindAuthorIndex<Runtime, Aura, u32>;
//! }
//! ```
//!
//! #### Defining block producer ID
//!
//! The pallet expects the Partner Chain to provide a type representing its block producers.
//! This type can be as simple as an Aura public key but can also be a more complex type if block producers
//! are not a homogenous group. For example, in the context of a Partner Chain using Ariadne committee selection,
//! it's typical to have two kinds of block producers: permissioned producers provided by the governance authority
//! and registered candidates recruited from among Cardano stake pool operators. In this instance an example
//! author type could be:
//! ```rust
//! use sidechain_domain::*;
//!
//! pub enum BlockAuthor {
//!     Incentivized(CrossChainPublicKey, StakePoolPublicKey),
//!     ProBono(CrossChainPublicKey),
//! }
//! ```
//!
//! Keep in mind that other Partner Chains SDK components put their own constraints on the block author type
//! that need to be adhered to for a Partner Chain to integrated them.
//!
//! #### Defining moment type
//!
//! The pallet abstracts away the notion of time when a block was produced and allows the chain builders to
//! configure it according to their chain's needs by providing a `Moment` type. This type can be a timestamp,
//! a slot or round number, depending on the consensus mechanism used.
//!
//! #### Support for adding to a running chain
//!
//! The pallet is written in a way that allows for adding it to an already live chain.
//!
//! ### Consuming the log
//!
//! **Important**: Consuming the log is a destructive operation. Multiple features should not consume the log data
//!                unless they are coordinated to read and clear the same log prefix.
//!
//! The pallet exposes three functions that allow other pallets to consume its data: `take_prefix`, `peek_prefix`
//! and `drop_prefix`. Any feature using the log should be able to identify the time up to which it should
//! process the log data and either:
//! - call `take_prefix` from some pallet's logic and process the returned data within the same block
//! - call `peek_prefix` inside an inherent data provider and use `drop_prefix` from the corresponding pallet
//!   to clear the previously peeked data within the same block
//!
//! It is critically important to drop exactly the prefix processed to avoid either skipping or double-counting some blocks.
//!
//! ## Usage - PC user
//!
//! This pallet does not expose any user-facing functionalities.
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

use core::marker::PhantomData;

pub use pallet::*;
pub use weights::WeightInfo;

/// Source of the current block's author
pub trait GetAuthor<BlockProducerId> {
	/// Returns the current block's author
	fn get_author() -> Option<BlockProducerId>;
}

/// [GetAuthor] implementation that uses a [FindAuthor] instance to get the current block's author index
/// of type `I` and uses it to read the author from `pallet_session_validator_management`.
pub struct FromFindAuthorIndex<T, FA, I>(PhantomData<(T, FA, I)>);

/// Source of the current block's moment
pub trait GetMoment<Moment> {
	/// Returns the current block's moment
	fn get_moment() -> Option<Moment>;
}

/// [GetMoment] implementation that fetches current block's `Moment` from storage `S`
pub struct FromStorage<S>(PhantomData<S>);

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Member;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// ID type that can represent any block producer in the network.
		/// This type should be defined by the Partner Chain Depending on its consensus mechanism and possible block producer types.
		type BlockProducerId: Member + Parameter + MaxEncodedLen;

		/// Type used to identify the moment in time when the block was produced, eg. a timestamp or slot number.
		type Moment: Member + Parameter + MaxEncodedLen + PartialOrd + Ord + PartialEq + Eq;

		/// Source of current block's author
		type GetAuthor: GetAuthor<Self::BlockProducerId>;

		/// Source of current block's moment
		type GetMoment: GetMoment<Self::Moment>;
	}

	#[pallet::storage]
	#[pallet::unbounded]
	pub type Log<T: Config> = StorageValue<_, Vec<(T::Moment, T::BlockProducerId)>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Block initialization hook that adds current block's author to the log
		fn on_initialize(block: BlockNumberFor<T>) -> Weight {
			let Some(author) = T::GetAuthor::get_author() else {
				log::warn!(
					"👷 Block production log update skipped - could not determine block {block:?} producer"
				);
				return T::DbWeight::get().reads(1);
			};
			let Some(moment) = T::GetMoment::get_moment() else {
				log::warn!(
					"👷 Block production log update skipped - could not determine block {block:?} time"
				);
				return T::DbWeight::get().reads(1);
			};

			log::info!("👷 Block {block:?} producer is {author:?}");
			Log::<T>::append((moment, author));

			T::DbWeight::get().reads_writes(2, 1)
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns all entries up to `moment` (inclusive) and removes them from the log
		pub fn take_prefix(moment: &T::Moment) -> Vec<(T::Moment, T::BlockProducerId)> {
			let removed_prefix = Log::<T>::mutate(|log| {
				let pos = log.partition_point(|(s, _)| s <= moment);
				log.drain(..pos).collect()
			});
			removed_prefix
		}

		/// Returns all entries up to `moment` (inclusive) from the log
		pub fn peek_prefix(
			moment: &T::Moment,
		) -> impl Iterator<Item = (T::Moment, T::BlockProducerId)> {
			Log::<T>::get().into_iter().take_while(move |(s, _)| s <= moment)
		}

		/// Removes all entries up to `moment` (inclusive) from the log
		pub fn drop_prefix(moment: &T::Moment) {
			Log::<T>::mutate(|log| {
				let position = log.partition_point(|(s, _)| s <= moment);
				log.drain(..position);
			});
		}
	}
}

#[cfg(feature = "block-participation")]
mod block_participation {
	use pallet_block_participation::BlockParticipationProvider;

	impl<T: crate::Config> BlockParticipationProvider<T::Moment, T::BlockProducerId>
		for crate::Pallet<T>
	{
		fn blocks_to_process(
			moment: &T::Moment,
		) -> impl Iterator<Item = (T::Moment, T::BlockProducerId)> {
			Self::peek_prefix(moment)
		}

		fn discard_processed_blocks(moment: &T::Moment) {
			Self::drop_prefix(moment)
		}
	}
}

mod source_impls {
	use super::*;
	use frame_support::{
		pallet_prelude::StorageValue,
		storage::types::QueryKindTrait,
		traits::{FindAuthor, StorageInstance},
	};
	use pallet_session_validator_management as psvm;
	use parity_scale_codec::FullCodec;
	use sp_runtime::traits::Get;

	impl<BlockProducerId, I, FA, T> GetAuthor<BlockProducerId> for FromFindAuthorIndex<T, FA, I>
	where
		FA: FindAuthor<I>,
		I: TryInto<usize>,
		T: psvm::Config,
		psvm::CommitteeMemberOf<T>: Into<BlockProducerId>,
	{
		fn get_author() -> Option<BlockProducerId> {
			Some(psvm::Pallet::<T>::find_current_authority::<I, FA>()?.into())
		}
	}

	impl<Prefix, Value, Moment, QueryKind, OnEmpty> GetMoment<Moment>
		for FromStorage<StorageValue<Prefix, Value, QueryKind, OnEmpty>>
	where
		Prefix: StorageInstance,
		Value: FullCodec,
		QueryKind: QueryKindTrait<Value, OnEmpty>,
		OnEmpty: Get<QueryKind::Query> + 'static,
		Option<Moment>: From<<QueryKind as QueryKindTrait<Value, OnEmpty>>::Query>,
	{
		fn get_moment() -> Option<Moment> {
			StorageValue::<Prefix, Value, QueryKind, OnEmpty>::get().into()
		}
	}
}
