//! Functions related to converting block number to slot/epoch/block-hash and vica-versa (when applicable)

use std::ops::Range;

pub use crate::tests::conversion::BEST_NUMBER;
pub use crate::tests::conversion::SLOTS_PER_EPOCH;
pub use crate::tests::conversion::block_hash_to_block_number;
pub use crate::tests::conversion::block_number_to_block_hash;

pub fn get_slot(block_number: u32) -> u32 {
	block_number
}

pub fn get_block_number(slot: u32) -> u32 {
	slot
}

pub fn get_epoch(block_number: u32) -> u32 {
	get_slot(block_number) / SLOTS_PER_EPOCH
}

pub fn get_first_block_in_epoch(epoch: u32) -> u32 {
	get_block_number(epoch * SLOTS_PER_EPOCH)
}

pub fn get_last_block_in_epoch(epoch: u32) -> u32 {
	get_block_number((epoch + 1) * SLOTS_PER_EPOCH - 1)
}

pub fn epoch_block_range(epoch: u32) -> Range<u32> {
	get_first_block_in_epoch(epoch)..get_last_block_in_epoch(epoch) + 1
}
