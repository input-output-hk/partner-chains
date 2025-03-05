#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod test;

use core::error::Error;
use parity_scale_codec::{Decode, Encode};
use sp_inherents::{InherentIdentifier, IsFatalError};
use sp_runtime::traits::Block as BlockT;
#[cfg(feature = "std")]
use {
	sp_api::{ApiExt, ProvideRuntimeApi},
	sp_inherents::InherentData,
};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"blprdlog";

#[derive(Encode, PartialEq)]
#[cfg_attr(not(feature = "std"), derive(Debug))]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error, sp_runtime::RuntimeDebug))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Block Author inherent must be provided every block"))]
	InherentRequired,
}
impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

#[cfg(feature = "std")]
#[derive(Debug)]
pub struct BlockAuthorInherentProvider<Author> {
	pub author: Option<Author>,
}

#[cfg(feature = "std")]
impl<Author> BlockAuthorInherentProvider<Author> {
	pub fn new<C, Member, Block>(
		client: &C,
		parent_hash: Block::Hash,
		slot: sidechain_slots::Slot,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	where
		Member: Decode,
		Block: BlockT,
		C: ProvideRuntimeApi<Block>,
		C::Api: BlockProductionLogApi<Block, Member>,
		Author: From<Member>,
	{
		if client
			.runtime_api()
			.has_api::<dyn BlockProductionLogApi<Block, Member>>(parent_hash)?
		{
			let author: Author = client.runtime_api().get_author(parent_hash, slot)?.into();

			Ok(BlockAuthorInherentProvider { author: Some(author) })
		} else {
			Ok(Self { author: None })
		}
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl<T> sp_inherents::InherentDataProvider for BlockAuthorInherentProvider<T>
where
	T: Send + Sync + Encode + Decode,
{
	async fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		if let Some(author) = &self.author {
			inherent_data.put_data(INHERENT_IDENTIFIER, author)?;
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
	pub trait BlockProductionLogApi<Member>
	where
		Member: Decode
	{
		fn get_author(slot: sidechain_slots::Slot) -> Member;
	}
}
