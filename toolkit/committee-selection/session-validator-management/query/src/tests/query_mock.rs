use super::conversion;
use crate::tests::conversion::BEST_NUMBER;
use derive_new::new;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, NumberFor, Zero};
use std::cell::OnceCell;

pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

pub struct TestApi {}

#[derive(new)]
pub struct TestRuntimeApi {
	#[new(default)]
	block_hash: OnceCell<Vec<u8>>,
}

impl TestRuntimeApi {
	pub fn check_using_same_instance_for_same_block(
		&self,
		hash_encoded: Vec<u8>,
	) -> Result<(), sp_api::ApiError> {
		if self.block_hash.get_or_init(|| hash_encoded.clone()) == &hash_encoded {
			Ok(())
		} else {
			Err(sp_api::ApiError::UsingSameInstanceForDifferentBlocks)
		}
	}
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = TestRuntimeApi;

	fn runtime_api(&self) -> ApiRef<Self::Api> {
		TestRuntimeApi::new().into()
	}
}

/// Blockchain database header backend. Does not perform any validation.
impl<Block: BlockT> HeaderBackend<Block> for TestApi
where
	<Block as BlockT>::Hash: From<[u8; 32]>,
{
	fn header(
		&self,
		_id: <Block as BlockT>::Hash,
	) -> Result<Option<Block::Header>, sp_blockchain::Error> {
		Ok(None)
	}

	fn info(&self) -> sp_blockchain::Info<Block> {
		sp_blockchain::Info {
			best_hash: conversion::block_number_to_block_hash(BEST_NUMBER).into(),
			best_number: BEST_NUMBER.into(),
			finalized_hash: Default::default(),
			finalized_number: Zero::zero(),
			genesis_hash: Default::default(),
			number_leaves: Default::default(),
			finalized_state: None,
			block_gap: None,
		}
	}

	fn status(
		&self,
		_id: <Block as BlockT>::Hash,
	) -> Result<sp_blockchain::BlockStatus, sp_blockchain::Error> {
		Ok(sp_blockchain::BlockStatus::Unknown)
	}

	fn number(&self, _hash: Block::Hash) -> Result<Option<NumberFor<Block>>, sp_blockchain::Error> {
		Ok(None)
	}

	fn hash(&self, number: NumberFor<Block>) -> Result<Option<Block::Hash>, sp_blockchain::Error> {
		let Ok(number): Result<u32, _> = number.try_into() else {
			panic!("this should never happen");
		};
		if number > conversion::BEST_NUMBER {
			Err(sp_blockchain::Error::UnknownBlock(number.to_string()))
		} else {
			let block_hash = conversion::block_number_to_block_hash(number);
			Ok(Some(block_hash.into()))
		}
	}
}
