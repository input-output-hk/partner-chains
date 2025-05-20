//! Binary search queries for Partner Chain slots and epochs
//!
//! # Purpose of this crate
//!
//! Standard Substrate block storage allows for retrieving blocks based on their number and hash.
//! However, Partner Chains toolkit introduces two new categories that are not supported by this
//! lookup: slot and epoch. This crate provides a mechanism to quickly query for blocks based on
//! their Partner Chain epoch or slot by applying a binary search over historical blocks.
//!
//! # Usage
//!
//! The binary search feature is provided via the [FindSidechainBlock] trait. This trait is
//! implemented for any runtime client that implements the [GetSidechainStatus] runtime API.
//! To query the blockchain, a predicate must be passed to the query that defines the searched
//! block. Some predefined targets are defined in the [predicates] module, otherwise a new target
//! type can be defined by implementing the [CompareStrategy] trait.
//!
//! Given a runtime client that satisfies the trait bounds, the blockchain can be queried like this:
//!
//! ```rust
//! use sidechain_block_search::predicates::AnyBlockInEpoch;
//! use sidechain_block_search::{ FindSidechainBlock, Client };
//! use sidechain_domain::*;
//! use sp_api::ProvideRuntimeApi;
//! use sp_runtime::traits::{ Block as BlockT, NumberFor };
//! use sp_sidechain::GetSidechainStatus;
//!
//! fn query_example<B, C>(client: C)
//! where
//!     B: BlockT,
//!     NumberFor<B>: From<u32> + Into<u32>,
//!     C: ProvideRuntimeApi<B> + Client<B> + Send + Sync + 'static,
//!     C::Api: GetSidechainStatus<B>
//! {
//!     let search_target = AnyBlockInEpoch {
//!         epoch: ScEpochNumber(42)
//!     };
//!     let result = client.find_block(search_target);
//! }
//! ```

#![deny(missing_docs)]

mod binary_search;
mod impl_block_info;
mod impl_compare_strategy;
mod impl_find_block;

pub use binary_search::binary_search_by;

use sidechain_domain::{ScEpochNumber, ScSlotNumber};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::NumberFor;
#[allow(deprecated)]
use sp_sidechain::GetSidechainStatus;
use std::cmp::Ordering;
use std::ops::Range;

#[cfg(test)]
mod tests;

/// Types of binary search queries over Partner Chain blocks
pub mod predicates {
	use super::*;

	/// Query for any block in given Partner Chain epoch
	pub struct AnyBlockInEpoch {
		/// Queried Partner Chain epoch
		pub epoch: ScEpochNumber,
	}

	/// Query for the first block in given Partner Chain epoch
	pub struct FirstBlockInEpoch {
		/// Queried Partner Chain epoch
		pub epoch: ScEpochNumber,
	}

	/// Query for the last block in given Partner Chain epoch
	pub struct LastBlockInEpoch {
		/// Queried Partner Chain epoch
		pub epoch: ScEpochNumber,
	}

	/// Query for any block in given slot range
	pub struct AnyBlockInSlotRange {
		/// Queried slot range. Left-inclusive, right-exclusive
		pub slot_range: Range<ScSlotNumber>,
	}

	/// Query for the last block in given slot range with upper block number bound
	pub struct LatestBlockInSlotRange<Block: BlockT> {
		/// Queried slot range. Left-inclusive, right-exclusive
		pub slot_range: Range<ScSlotNumber>,
		/// Upper bound for the number of returned block
		pub latest_block: NumberFor<Block>,
	}
}
use predicates::*;

/// Runtime API client used by the block queries in this crate
pub trait Client<Block: BlockT>: HeaderBackend<Block> + ProvideRuntimeApi<Block> {}

impl<C: HeaderBackend<Block> + ProvideRuntimeApi<Block>, Block: BlockT> Client<Block> for C {}

/// Interface for retrieving information about slot and epoch of Partner Chain blocks
pub trait SidechainInfo<Block: BlockT>: Client<Block> {
	/// Error type
	type Error: std::error::Error;

	/// Finds the Partner Chain slot number for a given block number
	fn get_slot_of_block(
		&self,
		block_number: NumberFor<Block>,
	) -> Result<ScSlotNumber, Self::Error>;

	/// Finds the Partner Chain eopch number for a given block number
	fn get_epoch_of_block(
		&self,
		block_number: NumberFor<Block>,
	) -> Result<ScEpochNumber, Self::Error>;
}

/// Comparator used for binary searching the block history
///
/// Types implementing this trait represent some _search target_, which is to be found through
/// binary search over block history. Note that this search target can be a single block defined
/// by its _slot_ or some other monotonically increasing block property, or a _range_ of blocks
/// defined by a range of slots or other property.
pub trait CompareStrategy<Block: BlockT, BlockInfo: Client<Block>> {
	/// Error type
	type Error: std::error::Error;

	/// Compares a block against a search target.
	///
	/// # Returns
	/// - `Ok(Ordering::Less)` if the block is below the target
	/// - `Ok(Ordering::Equal)` if the block is at target
	/// - `Ok(Ordering::Greater)` if the block is above the target
	/// - `Err` if an error occured
	fn compare_block(
		&self,
		block: NumberFor<Block>,
		block_info: &BlockInfo,
	) -> Result<Ordering, Self::Error>;
}

/// Runtime client capable of finding Partner Chain blocks via binary search using some [CompareStrategy].
pub trait FindSidechainBlock<Block: BlockT, CS: CompareStrategy<Block, Self>>:
	Client<Block> + Sized
{
	/// Error type
	type Error: std::error::Error;

	/// Finds the number of the block satisfying `compare_strategy`
	fn find_block_number(&self, compare_strategy: CS) -> Result<NumberFor<Block>, Self::Error>;

	/// Finds the hash of the block satisfying `compare_strategy`
	fn find_block(&self, compare_strategy: CS) -> Result<Block::Hash, Self::Error>;
}
