use crate::McHashInherentError::StableBlockNotFound;
use main_chain_follower_api::{
	block::MainchainBlock, common::Timestamp as McTimestamp, BlockDataSource, DataSourceError,
};
use sidechain_domain::{byte_string::ByteString, McBlockHash, McBlockNumber, McEpochNumber};
use sp_blockchain::HeaderBackend;
use sp_consensus_slots::{Slot, SlotDuration};
use sp_inherents::{InherentData, InherentDataProvider, InherentIdentifier};
use sp_partner_chains_consensus_aura::inherent_digest::InherentDigest;
use sp_runtime::{
	traits::{Block as BlockT, Header as HeaderT, Zero},
	DigestItem,
};
use sp_timestamp::Timestamp;
use std::{error::Error, ops::Deref};

#[cfg(test)]
mod test;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"scmchash";
pub const MC_HASH_DIGEST_ID: [u8; 4] = *b"mcsh";

#[derive(Debug)]
pub struct McHashInherentDataProvider {
	mc_block: MainchainBlock,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum McHashInherentError {
	#[error("{0}")]
	DataSourceError(#[from] DataSourceError),
	#[error("Stable block not found at {0}. It means that the main chain wasn't producing blocks for a long time.")]
	StableBlockNotFound(Timestamp),
	#[error("Slot represents a timestamp bigger than of u64::MAX")]
	SlotTooBig,
	#[error(
	"Main chain state {0} referenced in imported block at slot {1} with timestamp {2} not found"
	)]
	McStateReferenceInvalid(McBlockHash, Slot, Timestamp),
	#[error(
	"Main chain state {0} referenced in imported block at slot {1} corresponds to main chain block number which is lower than its parent's {2}<{3}"
	)]
	McStateReferenceRegressed(McBlockHash, Slot, McBlockNumber, McBlockNumber),
	#[error("Failed to retrieve MC hash from digest: {0}")]
	DigestError(String),
}

impl From<MainchainBlock> for McHashInherentDataProvider {
	fn from(mc_block: MainchainBlock) -> Self {
		Self { mc_block }
	}
}

impl Deref for McHashInherentDataProvider {
	type Target = MainchainBlock;

	fn deref(&self) -> &Self::Target {
		&self.mc_block
	}
}

impl McHashInherentDataProvider {
	pub async fn new_proposal(
		data_source: &(dyn BlockDataSource + Send + Sync),
		slot: Slot,
		slot_duration: SlotDuration,
	) -> Result<Self, McHashInherentError> {
		let slot_start_timestamp =
			slot_starting_timestamp(slot, slot_duration).ok_or(McHashInherentError::SlotTooBig)?;
		let mc_block = data_source
			.get_latest_stable_block_for(McTimestamp(slot_start_timestamp.as_millis()))
			.await?
			.ok_or(StableBlockNotFound(slot_start_timestamp))?;

		Ok(Self { mc_block })
	}

	pub async fn new_verification<Header>(
		parent_header: Header,
		parent_slot: Option<Slot>,
		verified_block_slot: Slot,
		mc_state_reference_hash: McBlockHash,
		slot_duration: SlotDuration,
		block_source: &(dyn BlockDataSource + Send + Sync),
	) -> Result<Self, McHashInherentError>
	where
		Header: HeaderT,
	{
		let mc_state_reference_block = get_mc_state_reference(
			verified_block_slot,
			mc_state_reference_hash.clone(),
			slot_duration,
			block_source,
		)
		.await?;

		let Some(parent_slot) = parent_slot else {
			// genesis block doesn't contain MC reference
			return Ok(Self::from(mc_state_reference_block));
		};

		let parent_mc_hash = McHashInherentDigest::value_from_digest(&parent_header.digest().logs)
			.map_err(|err| McHashInherentError::DigestError(err.to_string()))?;
		let parent_mc_state_reference_block =
			get_mc_state_reference(parent_slot, parent_mc_hash, slot_duration, block_source)
				.await?;

		if mc_state_reference_block.number < parent_mc_state_reference_block.number {
			Err(McHashInherentError::McStateReferenceRegressed(
				mc_state_reference_hash,
				verified_block_slot,
				mc_state_reference_block.number,
				parent_mc_state_reference_block.number,
			))
		} else {
			Ok(Self::from(mc_state_reference_block))
		}
	}

	pub fn mc_epoch(&self) -> McEpochNumber {
		self.mc_block.epoch
	}

	pub fn mc_block(&self) -> McBlockNumber {
		self.mc_block.number
	}

	pub fn mc_hash(&self) -> McBlockHash {
		self.mc_block.hash.clone()
	}
}

async fn get_mc_state_reference(
	verified_block_slot: Slot,
	verified_block_mc_hash: McBlockHash,
	slot_duration: SlotDuration,
	data_source: &(dyn BlockDataSource + Send + Sync),
) -> Result<MainchainBlock, McHashInherentError> {
	let timestamp = slot_starting_timestamp(verified_block_slot, slot_duration)
		.ok_or(McHashInherentError::SlotTooBig)?;
	data_source
		.get_stable_block_for(verified_block_mc_hash.clone(), McTimestamp(timestamp.as_millis()))
		.await
		.map_err(McHashInherentError::DataSourceError)?
		.ok_or(McHashInherentError::McStateReferenceInvalid(
			verified_block_mc_hash,
			verified_block_slot,
			timestamp,
		))
}

fn slot_starting_timestamp(slot: Slot, slot_duration: SlotDuration) -> Option<Timestamp> {
	slot_duration.as_millis().checked_mul(*slot).map(Timestamp::new)
}

#[async_trait::async_trait]
impl InherentDataProvider for McHashInherentDataProvider {
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.mc_block.hash)
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		_error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if *identifier == INHERENT_IDENTIFIER {
			panic!("BUG: {:?} inherent shouldn't return any errors", INHERENT_IDENTIFIER)
		} else {
			None
		}
	}
}

pub struct McHashInherentDigest;

impl McHashInherentDigest {
	pub fn from_mc_block_hash(mc_block_hash: McBlockHash) -> Vec<DigestItem> {
		vec![DigestItem::PreRuntime(MC_HASH_DIGEST_ID, mc_block_hash.0.to_vec())]
	}
}

impl InherentDigest for McHashInherentDigest {
	type Value = McBlockHash;

	fn from_inherent_data(
		inherent_data: &InherentData,
	) -> Result<Vec<sp_runtime::DigestItem>, Box<dyn Error + Send + Sync>> {
		let mc_hash = inherent_data
			.get_data::<McBlockHash>(&INHERENT_IDENTIFIER)
			.map_err(|err| format!("Failed to retrieve main chain hash from inherent data: {err}"))?
			.ok_or("Main chain hash missing from inherent data".to_string())?;
		Ok(Self::from_mc_block_hash(mc_hash))
	}

	fn value_from_digest(
		digest: &[DigestItem],
	) -> Result<Self::Value, Box<dyn Error + Send + Sync>> {
		for item in digest {
			if let DigestItem::PreRuntime(id, data) = item {
				if *id == MC_HASH_DIGEST_ID {
					let data = data.clone().try_into().map_err(|_| {
						format!("Invalid MC hash referenced by block author in digest: {:?}\nMC hash must be exactly 32 bytes long.", ByteString(data.to_vec()))
					})?;
					return Ok(McBlockHash(data));
				}
			}
		}
		Err("Main chain block hash missing from digest".into())
	}
}

pub fn get_inherent_digest_value_for_block<ID: InherentDigest, Block: BlockT, C>(
	client: &C,
	block_hash: Block::Hash,
) -> Result<Option<ID::Value>, Box<dyn Error + Send + Sync>>
where
	C: HeaderBackend<Block>,
	Block::Hash: std::fmt::Debug,
{
	let header = (client.header(block_hash))
		.map_err(|err| format!("Failed to retrieve header for hash {block_hash:?}: {err:?}"))?
		.ok_or(format!("Header for hash {block_hash:?} does not exist"))?;

	if header.number().is_zero() {
		Ok(None)
	} else {
		let value = ID::value_from_digest(&header.digest().logs)
			.map_err(|err| format!("Failed to retrieve inherent digest from header: {err:?}"))?;
		Ok(Some(value))
	}
}

pub fn get_mc_hash_for_block<Block: BlockT, C>(
	client: &C,
	block_hash: Block::Hash,
) -> Result<Option<McBlockHash>, Box<dyn Error + Send + Sync>>
where
	C: HeaderBackend<Block>,
	Block::Hash: std::fmt::Debug,
{
	get_inherent_digest_value_for_block::<McHashInherentDigest, Block, C>(client, block_hash)
}

#[cfg(any(feature = "mock", test))]
pub mod mock {
	use super::*;

	pub struct MockMcHashInherentDataProvider {
		pub mc_hash: McBlockHash,
	}

	#[async_trait::async_trait]
	impl sp_inherents::InherentDataProvider for MockMcHashInherentDataProvider {
		async fn provide_inherent_data(
			&self,
			inherent_data: &mut InherentData,
		) -> Result<(), sp_inherents::Error> {
			inherent_data.put_data(INHERENT_IDENTIFIER, &self.mc_hash)
		}

		async fn try_handle_error(
			&self,
			_identifier: &InherentIdentifier,
			_error: &[u8],
		) -> Option<Result<(), sp_inherents::Error>> {
			None
		}
	}
}
