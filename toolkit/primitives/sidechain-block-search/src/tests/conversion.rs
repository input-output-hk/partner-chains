//! Functions related to converting block number to slot/epoch/block-hash and vica-versa (when applicable)

use std::ops::Range;

pub const SLOTS_PER_EPOCH: u32 = 6;

pub fn block_number_to_block_hash(block_number: u32) -> [u8; 32] {
	let mut block_hash: [u8; 32] = [0u8; 32];
	block_hash[28..32].copy_from_slice(&block_number.to_be_bytes());

	block_hash
}

pub fn block_hash_to_block_number(block_hash: [u8; 32]) -> u32 {
	let mut last_four_bytes = [0u8; 4];
	last_four_bytes.copy_from_slice(&block_hash[28..32]);

	u32::from_be_bytes(last_four_bytes)
}

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

pub fn get_any_block_in_epoch(epoch: u32) -> Range<u32> {
	get_first_block_in_epoch(epoch)..get_last_block_in_epoch(epoch) + 1
}
