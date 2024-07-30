use crate::common::Timestamp;
use crate::Result;
use async_trait::async_trait;
use serde::Serialize;
use sidechain_domain::{McBlockHash, McBlockNumber, McEpochNumber, McSlotNumber};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct MainchainBlock {
	pub number: McBlockNumber,
	pub hash: McBlockHash,
	pub epoch: McEpochNumber,
	pub slot: McSlotNumber,
	pub timestamp: u64, // seconds since UNIX_EPOCH
}

/// Queries about Cardano Blocks
#[async_trait]
pub trait BlockDataSource {
	async fn get_latest_block_info(&self) -> Result<MainchainBlock>;

	/// Query for the currently latest stable block with timestamp within the `allowable_range(reference_timestamp) = [reference_timestamp - seconds(max_slot_boundary), reference_timestamp - seconds(slot_boundary)]`
	/// where `max_slot_boundary` is `3 * security_parameter/active_slot_coeff` (`3k/f`) and `min_slot_boundary` is `security_parameter/active_slot_coeff` (`k/f`).
	/// # Arguments
	/// * `reference_timestamp` - restricts the timestamps of MC blocks
	///
	/// # Returns
	/// * `Some(block)` - the latest stable block, with timestamp in the allowable range
	/// * `None` - none of the blocks is stable, and with timestamp valid in according to `reference_timestamp`
	async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>>;

	/// Find block by hash, filtered by block timestamp being in `allowable_range(reference_timestamp)`
	/// # Arguments
	/// * `hash` - the hash of the block
	/// * `reference_timestamp` - restricts the timestamp of the MC block
	///
	/// # Returns
	/// * `Some(block)` - the block with given hash, with timestamp in the allowable range
	/// * `None` - no stable block with given hash and timestamp in the allowable range exists
	async fn get_stable_block_for(
		&self,
		hash: McBlockHash,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>>;
}
