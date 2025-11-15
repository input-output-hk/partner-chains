use crate::{DataSourceError, Result};
use blockfrost_openapi::models::block_content::BlockContent;
use sidechain_domain::*;

pub fn from_block_content(value: BlockContent) -> Result<MainchainBlock> {
	Ok(MainchainBlock {
		number: value
			.height
			.map(|n| sidechain_domain::McBlockNumber(n as u32))
			.ok_or(DataSourceError::InvalidData("number missing".to_string()))?,
		hash: McBlockHash::decode_hex(&value.hash)?,
		epoch: value
			.epoch
			.map(|n| sidechain_domain::McEpochNumber(n as u32))
			.ok_or(DataSourceError::InvalidData("epoch missing".to_string()))?,
		slot: value
			.slot
			.map(|n| sidechain_domain::McSlotNumber(n as u64))
			.ok_or(DataSourceError::InvalidData("slot missing".to_string()))?,
		timestamp: value.time as u64,
	})
}
