//! Crate implementing a mechanism for Partner Chain blocks to reference a stable Cardano main chain block.
//!
//! # Purpose of this crate
//!
//! This crate provides a base for all other Partner Chain Toolkit's features that require Cardano observability.
//! Whenever some part of Cardano state is observed and included in a Partner Chain's own state, all Partner Chain
//! nodes need to agree on what data was eligible for inclusion when a PC block was produced. This is
//! best achieved by first agreeing on a stable Cardano block which then serves as the upper boundary of all observed
//! Cardano data. This _main chain reference block_ is selected by the block producing node and must be accepted by
//! other nodes as being stable at the time of block production.
//!
//! This crate implements the [InherentDataProvider] and [InherentDigest] mechanism for selecting a reference main
//! chain block as part of inherent data creation step and saving it as a _main chain reference hash_ in the block
//! header.
//!
//! # Prerequisites
//!
//! This feature uses the [InherentDigest] mechanism from [sp_partner_chains_consensus_aura] crate for storing inherent
//! data in the block header. Your node must use the [PartnerChainsProposer] defined by that crate for this feature to work.
//!
//! # Adding to the node
//!
//! To add the feature to your Partner Chain node, follow the instructions below.
//! Refer to the demo node implementation for reference.
//!
//! ## Data source
//!
//! A data source implementing the [McHashDataSource] interface trait should be added to your node. A Db-Sync
//! implementation can be found in the `partner-chains-db-sync-data-sources` crate. Refer to its documentation on
//! how to configure and use it.
//!
//! ## Adding the inherent data provider
//!
//! [McHashInherentDataProvider] should be added to your inherent data provider stack for both block proposal and
//! verification, using dedicated constructor for each.
//!
//! The main chain reference hash provided by [McHashInherentDataProvider] is meant to be used by other inherent data
//! providers from the Partner Chains Toolkit which require Cardano observability. As such, it should be created first in
//! your node's IDP stack, so that [McHashInherentDataProvider::mc_hash] and [McHashInherentDataProvider::previous_mc_hash]
//! can be used to pass the reference hashes to other IDPs.
//!
//! As an example, creation for proposal would look like the following:
//! ```rust
//! # use std::sync::Arc;
//! # use sidechain_mc_hash::*;
//! use sp_consensus_slots::{Slot, SlotDuration};
//! use sp_runtime::traits::Block as BlockT;
//!
//! struct ProposalCIDP {
//!     // the data source should be created as part of your node's setup
//!     mc_hash_data_source: Arc<dyn McHashDataSource + Send + Sync>,
//!     // slot duration should either be a part of your node's configuration or be
//!     // retrieved using `parent_hash` and your consensus mechanism's runtime API
//!     slot_duration: SlotDuration,
//! }
//!
//! #[async_trait::async_trait]
//! impl<Block> sp_inherents::CreateInherentDataProviders<Block, ()> for ProposalCIDP
//! where
//!     Block: BlockT + Send + Sync
//! {
//!     type InherentDataProviders = McHashInherentDataProvider;
//!
//!     async fn create_inherent_data_providers(
//!         &self,
//!         parent_hash: <Block as BlockT>::Hash,
//!         _extra_args: (),
//!     ) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
//!         let parent_header: <Block as BlockT>::Header = unimplemented!("Retrieved from block store");
//!         let slot: Slot = unimplemented!("Provided by the consensus IDP");
//!
//!         let mc_hash = McHashInherentDataProvider::new_proposal(
//!             parent_header,
//!             self.mc_hash_data_source.as_ref(),
//!             slot,
//!             self.slot_duration,
//!         ).await?;
//!
//!         // Other providers can now use mc_hash.mc_hash() and mc_hash.previous_mc_hash()
//!
//!         Ok((mc_hash /* other inherent data providers */))
//!     }
//! }
//! ```
//!
//! For block validation, creation of the IDP would look like this:
//! ```rust
//! # use std::sync::Arc;
//! # use sidechain_mc_hash::*;
//! # use sp_consensus_slots::{Slot, SlotDuration};
//! # use sp_runtime::traits::Block as BlockT;
//! use sidechain_domain::McBlockHash;
//!
//! struct VerificationIDP {
//!     // the data source should be created as part of your node's setup
//!     mc_hash_data_source: Arc<dyn McHashDataSource + Send + Sync>,
//!     // slot duration should either be a part of your node's configuration or be
//!     // retrieved using `parent_hash` and your consensus mechanism's runtime API
//!     slot_duration: SlotDuration,
//! }
//!
//! #[async_trait::async_trait]
//! impl<Block> sp_inherents::CreateInherentDataProviders<Block, (Slot, McBlockHash)> for VerificationIDP
//! where
//!     Block: BlockT + Send + Sync
//! {
//!     type InherentDataProviders = McHashInherentDataProvider;
//!
//!     async fn create_inherent_data_providers(
//!         &self,
//!         parent_hash: <Block as BlockT>::Hash,
//!	        (block_slot_from_header, mc_hash_from_header): (Slot, McBlockHash),
//!     ) -> Result<Self::InherentDataProviders, Box<dyn std::error::Error + Send + Sync>> {
//!         let parent_header: <Block as BlockT>::Header = unimplemented!("Retrieved from block store");
//!         let slot: Slot = unimplemented!("Provided by the consensus IDP");
//!         let parent_slot: Option<Slot> = unimplemented!("Read from previous block using runtime API");
//!
//!         let mc_hash = McHashInherentDataProvider::new_verification(
//!             parent_header,
//!             parent_slot,
//!             block_slot_from_header,
//!             mc_hash_from_header,
//!             self.slot_duration,
//!             self.mc_hash_data_source.as_ref(),
//!         )
//!         .await?;
//!
//!         // Other providers can now use mc_hash.mc_hash() and mc_hash.previous_mc_hash()
//!
//!         Ok((mc_hash /* other inherent data providers */))
//!     }
//! }
//! ```
//!
//! Note that for verification the implementation now accepts additional arguments of types [Slot] and [McBlockHash]
//! which are provided by the import queue based on the header. In this case, the [McBlockHash] value comes from the
//! [InherentDigest] defined in next section.
//!
//! ## Import queue configuration
//!
//! To be able to save and retrieve inherent data in the block header, your node should use the [PartnerChainsProposer]
//! instead of the stock proposer provided by Substrate. Refer to its documentation for more information.
//!
//! For this feature to work, the only modification needed is to set [McHashInherentDigest] as the inherent digest
//! type of the proposer. This will cause it to save the inherent data provided by [McHashInherentDataProvider] to be
//! also saved in the block header and available for inspection and verification. Assuming your node uses
//! [PartnerChainsProposerFactory], this should look like the following:
//!
//! ```rust
//! use sidechain_mc_hash::McHashInherentDigest;
//! use sp_consensus::Environment;
//! use sp_partner_chains_consensus_aura::block_proposal::PartnerChainsProposerFactory;
//! use sp_runtime::traits::Block as BlockT;
//!
//! fn new_full<Block: BlockT, ProposerFactory: Environment<Block>>(
//! 	base_proposer_factory: ProposerFactory,
//! ) {
//!     // .. other node setup logic
//! 	let proposer_factory: PartnerChainsProposerFactory<
//! 		Block,
//! 		ProposerFactory,
//! 		McHashInherentDigest,
//! 	> = PartnerChainsProposerFactory::new(base_proposer_factory);
//!     // ..
//! }
//! ```
//!
//! [PartnerChainsProposer]: sp_partner_chains_consensus_aura::block_proposal::PartnerChainsProposer
//! [PartnerChainsProposerFactory]: sp_partner_chains_consensus_aura::block_proposal::PartnerChainsProposerFactory

#![warn(missing_docs)]
use crate::McHashInherentError::StableBlockNotFound;
use async_trait::async_trait;
use sidechain_domain::{byte_string::ByteString, *};
use sp_blockchain::HeaderBackend;
use sp_consensus_slots::{Slot, SlotDuration};
use sp_inherents::{InherentData, InherentDataProvider, InherentIdentifier};
use sp_partner_chains_consensus_aura::inherent_digest::InherentDigest;
use sp_runtime::{
	DigestItem,
	traits::{Block as BlockT, Header as HeaderT, Zero},
};
use sp_timestamp::Timestamp;
use std::{error::Error, ops::Deref};

#[cfg(test)]
mod test;

/// Inherent identifier under which the main chain block reference is provided
///
/// Data under this ID is not used by any pallet and is instead put in the block
/// header under [MC_HASH_DIGEST_ID] using [InherentDigest].
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"scmchash";

/// Digest ID of the main chain reference block hash
///
/// Inherent data provided for [INHERENT_IDENTIFIER] is saved with this digest ID
/// in the block header using [InherentDigest].
pub const MC_HASH_DIGEST_ID: [u8; 4] = *b"mcsh";

/// Inherent data provider that provides the hash of the latest stable Cardano block
/// (main chain reference block) observed by the block producer to be included in the
/// header of the currently produced PC block.
///
/// This IDP also exposes further information about the currently referenced Cardano
/// block and the block referenced by the parent block of the current one, for use by
/// other inherent data providers.
///
/// ## Creation
///
/// This inherent data provider has two different constructors that should be used
/// depending on whether it is being used for proposing or verifying a block:
/// - [new_proposal][McHashInherentDataProvider::new_proposal] which selects the main
///   chain reference block
/// - [new_verification][McHashInherentDataProvider::new_verification] which verifies
///   the main chain reference in the block header
#[derive(Debug)]
pub struct McHashInherentDataProvider {
	mc_block: MainchainBlock,
	previous_mc_block: Option<MainchainBlock>,
}

/// Error type returned by constructors of [McHashInherentDataProvider]
#[derive(Debug, thiserror::Error)]
pub enum McHashInherentError {
	/// Signals that the data source returned an error
	#[error("{0}")]
	DataSourceError(Box<dyn Error + Send + Sync>),
	/// Signals that no stable Cardano block was found within timestamp constraints
	#[error(
		"Stable block not found at {0}. It means that the main chain wasn't producing blocks for a long time."
	)]
	StableBlockNotFound(Timestamp),
	/// Signals that a slot beyond supported range of [u64] was passed
	#[error("Slot represents a timestamp bigger than of u64::MAX")]
	SlotTooBig,
	/// Signals that a Cardano block referenced by a main chain reference hash could not be found
	#[error(
		"Main chain state {0} referenced in imported block at slot {1} with timestamp {2} not found"
	)]
	McStateReferenceInvalid(McBlockHash, Slot, Timestamp),
	/// Signals that a main chain reference hash points to a Cardano block earlier than the one referenced
	/// by the previous Partner Chain block
	#[error(
		"Main chain state {0} referenced in imported block at slot {1} corresponds to main chain block number which is lower than its parent's {2}<{3}"
	)]
	McStateReferenceRegressed(McBlockHash, Slot, McBlockNumber, McBlockNumber),
	/// Signals that a main chain reference hash is either missing from the block diget or can not be decoded
	#[error("Failed to retrieve MC hash from digest: {0}")]
	DigestError(String),
	/// Signals that block details could not be retrieved for a stable Cardano block
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

/// Data source API used by [McHashInherentDataProvider]
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
	/// Creates a new [McHashInherentDataProvider] for proposing a new Partner Chain block by querying the
	/// current state of Cardano and returning an instance referencing the latest stable block there.
	///
	/// # Arguments
	/// - `parent_header`: header of the parent of the block being produced
	/// - `data_source`: data source implementing [McHashDataSource]
	/// - `slot`: current Partner Chain slot
	/// - `slot_duration`: duration of the Partner Chain slot
	///
	/// The referenced Cardano block is guaranteed to be later or equal to the one referenced by the parent block
	/// and within a bounded time distance from `slot`.
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

	/// Verifies a Cardano reference hash and creates a new [McHashInherentDataProvider] for an imported Partner Chain block.
	///
	/// # Arguments
	/// - `parent_header`: header of the parent of the block being produced or validated
	/// - `parent_slot`: slot of the parent block. [None] for genesis
	/// - `verified_block_slot`: Partner Chain slot of the block being verified
	/// - `mc_state_reference_hash`: Cardano block hash referenced by the block being verified
	/// - `slot_duration`: duration of the Partner Chain slot
	/// - `block_source`: data source implementing [McHashDataSource]
	///
	/// # Returns
	/// This function will return an error if `mc_state_reference_hash` is not found or is before the block referenced by
	/// the parent of the block being verified.
	///
	/// Otherwise, the returned [McHashInherentDataProvider] instance will contain block data for `mc_state_reference_hash`.
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

	/// Returns the Cardano epoch containing the reference block
	pub fn mc_epoch(&self) -> McEpochNumber {
		self.mc_block.epoch
	}

	/// Returns the reference block's number
	pub fn mc_block(&self) -> McBlockNumber {
		self.mc_block.number
	}

	/// Returns the reference block's hash
	pub fn mc_hash(&self) -> McBlockHash {
		self.mc_block.hash.clone()
	}

	/// Returns the previous reference block's hash
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

/// [InherentDigest] implementation for the main chain reference hash
pub struct McHashInherentDigest;

impl McHashInherentDigest {
	/// Creates a [DigestItem] containing the given main chain reference hash
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

#[allow(missing_docs)]
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

#[allow(missing_docs)]
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

/// Mock implementations of components in this crate for use in tests
#[cfg(any(feature = "mock", test))]
pub mod mock {
	use super::*;
	use derive_new::new;

	/// Mock implementation of [McHashInherentDataProvider] which always returns the same block
	pub struct MockMcHashInherentDataProvider {
		/// Main chain reference block to be returned by this mock
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

	/// Mock implementation of [McHashDataSource]
	#[derive(new, Clone)]
	pub struct MockMcHashDataSource {
		/// Stable blocks ordered from oldest to newest
		pub stable_blocks: Vec<MainchainBlock>,
		/// Unstable blocks ordered from oldest to newest
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
