use crate::McHashInherentError::StableBlockNotFound;
use async_trait::async_trait;
use sidechain_domain::{byte_string::ByteString, *};
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
	previous_mc_block: Option<MainchainBlock>,
}

#[derive(Debug, thiserror::Error)]
pub enum McHashInherentError {
	#[error("{0}")]
	DataSourceError(Box<dyn Error + Send + Sync>),
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
	#[error("Failed to retrieve MC Block that was verified as stable by its hash")]
	StableBlockNotFoundByHash(McBlockHash),
}

impl From<MainchainBlock> for McHashInherentDataProvider {
	fn from(mc_block: MainchainBlock) -> Self {
		Self { mc_block, previous_mc_block: None }
	}
}

impl Deref for McHashInherentDataProvider {
	type Target = MainchainBlock;

	fn deref(&self) -> &Self::Target {
		&self.mc_block
	}
}

/// Queries about Cardano Blocks
#[async_trait]
pub trait McHashDataSource {
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
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>>;

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
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>>;

	/// Find block by hash.
	/// # Arguments
	/// * `hash` - the hash of the block
	///
	/// # Returns
	/// * `Some(block)` - the block with given hash
	/// * `None` - no block with the given hash was found
	async fn get_block_by_hash(
		&self,
		hash: McBlockHash,
	) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>>;
}

impl McHashInherentDataProvider {
	pub async fn new_proposal<Header>(
		parent_header: Header,
		data_source: &(dyn McHashDataSource + Send + Sync),
		slot: Slot,
		slot_duration: SlotDuration,
	) -> Result<Self, McHashInherentError>
	where
		Header: HeaderT,
	{
		let slot_start_timestamp =
			slot.timestamp(slot_duration).ok_or(McHashInherentError::SlotTooBig)?;
		let mc_block = data_source
			.get_latest_stable_block_for(slot_start_timestamp)
			.await
			.map_err(McHashInherentError::DataSourceError)?
			.ok_or(StableBlockNotFound(slot_start_timestamp))?;

		match McHashInherentDigest::value_from_digest(&parent_header.digest().logs).ok() {
			// If parent block references some MC state, it is illegal to propose older state
			Some(parent_mc_hash) => {
				let parent_stable_mc_block = data_source
					.get_block_by_hash(parent_mc_hash.clone())
					.await
					.map_err(McHashInherentError::DataSourceError)?
					.ok_or(McHashInherentError::StableBlockNotFoundByHash(parent_mc_hash))?;

				if mc_block.number >= parent_stable_mc_block.number {
					Ok(Self { mc_block, previous_mc_block: Some(parent_stable_mc_block) })
				} else {
					Ok(Self {
						mc_block: parent_stable_mc_block.clone(),
						previous_mc_block: Some(parent_stable_mc_block),
					})
				}
			},
			None => Ok(Self { mc_block, previous_mc_block: None }),
		}
	}

	pub async fn new_verification<Header>(
		parent_header: Header,
		parent_slot: Option<Slot>,
		verified_block_slot: Slot,
		mc_state_reference_hash: McBlockHash,
		slot_duration: SlotDuration,
		block_source: &(dyn McHashDataSource + Send + Sync),
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
			Ok(Self {
				mc_block: mc_state_reference_block,
				previous_mc_block: Some(parent_mc_state_reference_block),
			})
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

	pub fn previous_mc_hash(&self) -> Option<McBlockHash> {
		self.previous_mc_block.as_ref().map(|block| block.hash.clone())
	}
}

async fn get_mc_state_reference(
	verified_block_slot: Slot,
	verified_block_mc_hash: McBlockHash,
	slot_duration: SlotDuration,
	data_source: &(dyn McHashDataSource + Send + Sync),
) -> Result<MainchainBlock, McHashInherentError> {
	let timestamp = verified_block_slot
		.timestamp(slot_duration)
		.ok_or(McHashInherentError::SlotTooBig)?;
	data_source
		.get_stable_block_for(verified_block_mc_hash.clone(), timestamp)
		.await
		.map_err(McHashInherentError::DataSourceError)?
		.ok_or(McHashInherentError::McStateReferenceInvalid(
			verified_block_mc_hash,
			verified_block_slot,
			timestamp,
		))
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
	use derive_new::new;

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

	#[derive(new, Clone)]
	pub struct MockMcHashDataSource {
		pub stable_blocks: Vec<MainchainBlock>,
		pub unstable_blocks: Vec<MainchainBlock>,
	}

	impl From<Vec<MainchainBlock>> for MockMcHashDataSource {
		fn from(stable_blocks: Vec<MainchainBlock>) -> Self {
			Self { stable_blocks, unstable_blocks: vec![] }
		}
	}

	#[async_trait]
	impl McHashDataSource for MockMcHashDataSource {
		async fn get_latest_stable_block_for(
			&self,
			_reference_timestamp: Timestamp,
		) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
			Ok(self.stable_blocks.last().cloned())
		}

		async fn get_stable_block_for(
			&self,
			hash: McBlockHash,
			_reference_timestamp: Timestamp,
		) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
			Ok(self.stable_blocks.iter().find(|b| b.hash == hash).cloned())
		}

		async fn get_block_by_hash(
			&self,
			hash: McBlockHash,
		) -> Result<Option<MainchainBlock>, Box<dyn std::error::Error + Send + Sync>> {
			Ok(self
				.stable_blocks
				.iter()
				.find(|b| b.hash == hash)
				.cloned()
				.or_else(|| self.unstable_blocks.iter().find(|b| b.hash == hash).cloned()))
		}
	}
}
