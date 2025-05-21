mod sidechain;

pub use sidechain::*;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

/// Blockchain client for retrieving the latest Partner Chain block hash
pub trait GetBestHash<Block: BlockT> {
	/// Returns the latest Partner Chain block hash
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
