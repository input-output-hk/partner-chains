use super::Block;
use crate::{SidechainRpcClient, mock::mock_utxo_id};
use sidechain_domain::UtxoId;
use sp_blockchain::HeaderBackend;
use sp_core::offchain::Duration;
use sp_runtime::traits::{Block as BlockT, NumberFor, Zero};

#[derive(Clone)]
pub struct TestApi {
	pub epoch_duration: u64,
	#[cfg(feature = "legacy-slotapi-compat")]
	pub slot_duration: Option<u64>,
}

impl Default for TestApi {
	fn default() -> Self {
		Self {
			epoch_duration: 6000,
			#[cfg(feature = "legacy-slotapi-compat")]
			slot_duration: None,
		}
	}
}

impl SidechainRpcClient<Block> for TestApi {
	fn get_epoch_duration(
		&self,
		_best_block: <Block as BlockT>::Hash,
	) -> Result<sp_core::offchain::Duration, Box<dyn std::error::Error + Send + Sync>> {
		Ok(Duration::from_millis(self.epoch_duration))
	}

	fn get_genesis_utxo(
		&self,
		_best_block: <Block as BlockT>::Hash,
	) -> Result<UtxoId, Box<dyn std::error::Error + Send + Sync>> {
		Ok(mock_utxo_id())
	}

	#[cfg(feature = "legacy-slotapi-compat")]
	fn get_maybe_slot_duration(&self, _best_block: <Block as BlockT>::Hash) -> Option<u64> {
		self.slot_duration.clone()
	}
}

/// Blockchain database header backend. Does not perform any validation.
impl<Block: BlockT> HeaderBackend<Block> for TestApi
where
	<Block as BlockT>::Hash: From<[u8; 32]>,
	<NumberFor<Block> as TryInto<u64>>::Error: std::fmt::Debug,
{
	fn header(
		&self,
		_id: <Block as BlockT>::Hash,
	) -> Result<Option<Block::Header>, sp_blockchain::Error> {
		Ok(None)
	}

	fn info(&self) -> sp_blockchain::Info<Block> {
		sp_blockchain::Info {
			// The higher this is, the longer it takes for RPC `sidechain_getSignaturesToUpload` to run
			best_hash: Default::default(),
			best_number: Default::default(),
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

	fn hash(&self, _number: NumberFor<Block>) -> Result<Option<Block::Hash>, sp_blockchain::Error> {
		Ok(Some(Default::default()))
	}
}
