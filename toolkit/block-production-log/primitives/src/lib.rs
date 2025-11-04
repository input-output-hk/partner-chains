//! Primitives and support crate for `pallet_block_production_log`.
//!
//! This crate defines the primitive types and the inherent data provider for the block production log feature.
//!
//! ## Usage
//!
//! This crate supports operation of `pallet_block_production_log`.
//! Consult the pallet's documentation on how to include it in the runtime.
//!
//! ### Adding to the node
//!
//! #### Defining the types
//!
//! The feature requires two types to be defind by the chain builders in their code:
//! - `Author`: the type representing the block author, as it's configured in the feature's pallet
//! - `Moment`: a moment in time when the block was produced, which carries enough information to
//!             calculate the block's author. Typcally, this type can be a timestamp, or a slot,
//!             depending on the consensus mechanism used, but can be a richer type if needed.
//!
//! #### Implementing the runtime API
//!
//! The block production log feature requires [BlockProductionLogApi] to be implemented by the Partner Chain runtime
//! so the current block author can be identified. The concrete implementation must use or match the mechanism of
//! block author selection used by the particular Partner Chain's consensus mechanism.
//!
//! An example for a Partner Chain using Aura consensus looks like this:
//! ```rust, ignore
//! impl_runtime_apis! {
//!     impl BlockProductionLogApi<Block, CommitteeMember, Slot> for Runtime {
//!         fn get_author(slot: Slot) -> Option<CommitteeMember> {
//!             SessionCommitteeManagement::get_current_authority_round_robin(u64::from(*slot) as usize)
//!         }
//!     }
//! }
//! ```
//! using the `pallet_session_committee_management::Pallet::get_current_authority_round_robin` function
//! which performs the same round-robin author selection that Aura does internally.
//!
//! #### Adding the inherent data provider
//!
//! The inherent data provider should be added to the node's `CreateInherentDataProviders` implementation for
//! both proposal and validation of blocks.  eg.:
//!
//! ```rust,ignore
//! // Create the inherent data provider. `slot` must be the slot number of the block currently being produced/verified
//! let block_author_idp = BlockAuthorInherentProvider::new(client.as_ref(), parent_hash, slot)?;
//! ...
//! // Return the inherent data provider together with other IDPs
//! Ok((timestamp_idp, slot_idp, ..., block_author_idp, ...))
//! ```
//!
//! Notice that the IDP must be passed the current `Moment`, which in this instance is `Slot`.
//!
//! The inherent data provider created using `BlockAuthorInherentProvider::new` will check whether `BlockProductionLogApi`
//! is available in the runtime and will only provide inherent data if the API is present.
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

#[cfg(test)]
mod test;

use parity_scale_codec::{Decode, Encode};
use sp_inherents::{InherentIdentifier, IsFatalError};
#[cfg(feature = "std")]
use {
	sp_api::{ApiExt, ProvideRuntimeApi},
	sp_inherents::InherentData,
	sp_runtime::traits::Block as BlockT,
};

/// Inherent identifier used by the Block Production Log pallet
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"blprdlog";

/// Error type used for failing calls of the block production log feature's inherent.
#[derive(Encode, PartialEq)]
#[cfg_attr(not(feature = "std"), derive(Debug))]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error, sp_runtime::RuntimeDebug))]
pub enum InherentError {
	/// Inherent was not produced when expected
	#[cfg_attr(
		feature = "std",
		error("Block Author inherent must be provided every block after initialization")
	)]
	InherentRequired,
	/// Block Author inherent data is not correctly encoded
	#[cfg_attr(feature = "std", error("Block Author inherent data is not correctly encoded"))]
	InvalidInherentData,
}
impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

/// Current block producer data used as inherent data
#[derive(Clone, Debug, Encode, Decode, PartialEq, Eq)]
pub struct BlockProductionInherentDataV1<Moment, BlockProducerId> {
	/// Moment of the block
	pub moment: Moment,
	/// Block producer ID
	pub block_producer_id: BlockProducerId,
}

/// Inherent data provider providing the block author of the current block
/// Type parameters:
/// - `Author`: Type representing a block author.
#[cfg(feature = "std")]
#[derive(Debug)]
pub struct BlockAuthorInherentProvider<Moment, Author> {
	/// Optional value of the current block author. Inherent data is not provided if [None].
	pub data: Option<BlockProductionInherentDataV1<Moment, Author>>,
}

#[cfg(feature = "std")]
impl<Moment, Author> BlockAuthorInherentProvider<Moment, Author> {
	/// Creates a new [BlockAuthorInherentProvider] using runtime API [BlockProductionLogApi].
	///
	/// The inherent data provider returned will be inert if [BlockProductionLogApi] is not detected in the runtime.
	pub fn new<C, Block>(
		client: &C,
		parent_hash: Block::Hash,
		moment: Moment,
	) -> Result<Self, Box<dyn core::error::Error + Send + Sync>>
	where
		Moment: Encode,
		Author: Decode,
		Block: BlockT,
		C: ProvideRuntimeApi<Block>,
		C::Api: BlockProductionLogApi<Block, Author, Moment>,
	{
		let api = client.runtime_api();
		if !api.has_api::<dyn BlockProductionLogApi<Block, Author, Moment>>(parent_hash)? {
			return Ok(Self { data: None });
		}
		let data = client
			.runtime_api()
			.get_author(parent_hash, &moment)?
			.map(|author| BlockProductionInherentDataV1 { moment, block_producer_id: author });

		Ok(BlockAuthorInherentProvider { data })
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl<Moment, T> sp_inherents::InherentDataProvider for BlockAuthorInherentProvider<Moment, T>
where
	T: Send + Sync + Encode + Decode,
	Moment: Send + Sync + Encode + Decode,
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		if let Some(data) = &self.data {
			inherent_data.put_data(INHERENT_IDENTIFIER, data)?;
		}
		Ok(())
	}

	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		mut error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		if identifier == &INHERENT_IDENTIFIER {
			let error = InherentError::decode(&mut error).ok()?;
			Some(Err(sp_inherents::Error::Application(Box::from(error))))
		} else {
			None
		}
	}
}

sp_api::decl_runtime_apis! {
	/// Runtime API exposing data required for the [BlockAuthorInherentProvider] to operate.
	/// Type parameters:
	/// - `Author`: type representing a committee member eligible to be a block author. This type should correspond
	///             to what is configured as the block author type used by the pallet.
	pub trait BlockProductionLogApi<Author, Moment>
	where
		Author: Decode,
		Moment: Encode
	{
		/// Function returning the current block's author.
		///
		/// Its implementation must either use data exposed by the consensus mechanism used by the Partner Chain,
		/// independently calculate it, or obtain it from another source.
		///
		/// Parameters:
		/// - `moment`: value representing the current block's moment in time. It will typically be a timestamp or
		///             slot but can be any other value that is needed to decide the current block's author and can
		///             be passed from the node.
		fn get_author(moment: &Moment) -> Option<Author>;
	}
}
