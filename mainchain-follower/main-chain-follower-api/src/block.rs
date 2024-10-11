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
