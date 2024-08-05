//! Functions related to converting block number to slot/epoch/block-hash and vica-versa (when applicable)

pub const BLOCK_TO_SLOT_OFFSET: u32 = 102;

pub const GENESIS_EPOCH: u64 = 0;
pub const EPOCH_OF_BLOCK_1: u64 = (BLOCK_TO_SLOT_OFFSET / SLOTS_PER_EPOCH) as u64;
/// Latest epoch number
pub const BEST_EPOCH: u64 = 12345;

/// Latest block number
pub const BEST_NUMBER: u32 = ((BEST_EPOCH as u32 + 1) * SLOTS_PER_EPOCH - 1) - BLOCK_TO_SLOT_OFFSET;

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

pub const fn get_slot(block_number: u32) -> u32 {
	block_number + BLOCK_TO_SLOT_OFFSET
}

pub const fn get_epoch(block_number: u32) -> u32 {
	get_slot(block_number) / SLOTS_PER_EPOCH
}
