//! Binary Search Algorithm for discovering the Sidechain block number that contains the queried epoch

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
#[cfg(test)]
pub use tests::conversion;

pub mod predicates {
	use super::*;

	pub struct AnyBlockInEpoch {
		pub epoch: ScEpochNumber,
	}
	pub struct FirstBlockInEpoch {
		pub epoch: ScEpochNumber,
	}
	pub struct LastBlockInEpoch {
		pub epoch: ScEpochNumber,
	}
	pub struct AnyBlockInSlotRange {
		pub slot_range: Range<ScSlotNumber>,
	}
	pub struct LatestBlockInSlotRange<Block: BlockT> {
		pub slot_range: Range<ScSlotNumber>,
		pub latest_block: NumberFor<Block>,
	}
}
use predicates::*;

pub trait Client<Block: BlockT>: HeaderBackend<Block> + ProvideRuntimeApi<Block> {}

impl<C: HeaderBackend<Block> + ProvideRuntimeApi<Block>, Block: BlockT> Client<Block> for C {}

pub trait SidechainInfo<Block: BlockT>: Client<Block> {
	type Error: std::error::Error;

	fn get_slot_of_block(
		&self,
		block_number: NumberFor<Block>,
	) -> Result<ScSlotNumber, Self::Error>;
	fn get_epoch_of_block(
		&self,
		block_number: NumberFor<Block>,
	) -> Result<ScEpochNumber, Self::Error>;
}

pub trait CompareStrategy<Block: BlockT, BlockInfo: Client<Block>> {
	type Error: std::error::Error;

	fn compare_block(
		&self,
		block: NumberFor<Block>,
		block_info: &BlockInfo,
	) -> Result<Ordering, Self::Error>;
}

/// Find the sidechain block for a given sidechain epoch
pub trait FindSidechainBlock<Block: BlockT, CS: CompareStrategy<Block, Self>>:
	Client<Block> + Sized
{
	type Error: std::error::Error;

	fn find_block_number(&self, compare_strategy: CS) -> Result<NumberFor<Block>, Self::Error>;

	fn find_block(&self, compare_strategy: CS) -> Result<Block::Hash, Self::Error>;
}
