use super::conversion;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::{Block as BlockT, NumberFor, Zero};

// The build.rs file of `substrate_test_runtime` is throwing an error. So a `Block` is being manually defined
pub type Block = sp_runtime::generic::Block<
	sp_runtime::generic::Header<u32, sp_runtime::traits::BlakeTwo256>,
	sp_runtime::OpaqueExtrinsic,
>;

#[derive(Clone)]
pub struct TestClient {
	pub best_number: u32,
}

pub struct TestRuntimeApi {}

impl ProvideRuntimeApi<Block> for TestClient {
	type Api = TestRuntimeApi;

	fn runtime_api(&self) -> ApiRef<Self::Api> {
		TestRuntimeApi {}.into()
	}
}

/// Blockchain database header backend. Does not perform any validation.
impl<Block: BlockT> HeaderBackend<Block> for TestClient
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
			best_hash: conversion::block_number_to_block_hash(self.best_number).into(),
			best_number: self.best_number.into(),
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
		if number > self.best_number {
			Err(sp_blockchain::Error::UnknownBlock(number.to_string()))
		} else {
			let block_hash = conversion::block_number_to_block_hash(number);
			Ok(Some(block_hash.into()))
		}
	}
}
