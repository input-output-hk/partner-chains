mod sidechain;

pub use sidechain::*;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

pub trait GetBestHash<Block: BlockT> {
	fn best_hash(&self) -> Block::Hash;
}

impl<Block, T> GetBestHash<Block> for T
where
	T: HeaderBackend<Block>,
	Block: BlockT,
{
	fn best_hash(&self) -> Block::Hash {
		self.info().best_hash
	}
}
