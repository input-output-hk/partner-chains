use blockfrost_openapi::models::block_content::BlockContent;
use sidechain_domain::*;

pub fn from_block_content(value: BlockContent) -> Result<MainchainBlock, String> {
	Ok(MainchainBlock {
		number: value
			.height
			.map(|n| sidechain_domain::McBlockNumber(n as u32))
			.ok_or("number missing")?,
		hash: McBlockHash::decode_hex(&value.hash)?,
		epoch: value
			.epoch
			.map(|n| sidechain_domain::McEpochNumber(n as u32))
			.ok_or("epoch missing")?,
		slot: value
			.slot
			.map(|n| sidechain_domain::McSlotNumber(n as u64))
			.ok_or("slot missing")?,
		timestamp: value.time as u64,
	})
}
