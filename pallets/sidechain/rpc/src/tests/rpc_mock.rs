use super::Block;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_consensus_slots::SlotDuration;
use sp_runtime::traits::{Block as BlockT, NumberFor, Zero};
use sp_sidechain::SidechainStatus;

#[derive(Default)]
pub struct TestApi {
	pub runtime_api: TestRuntimeApi,
}

#[derive(Clone)]
pub struct TestRuntimeApi {
	pub sidechain_status: Vec<(<Block as BlockT>::Hash, SidechainStatus)>,
	pub slot_duration: SlotDuration,
	pub slots_per_epoch: u64,
}

impl Default for TestRuntimeApi {
	fn default() -> Self {
		Self {
			slot_duration: SlotDuration::from_millis(6000),
			slots_per_epoch: 10,
			sidechain_status: Default::default(),
		}
	}
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = TestRuntimeApi;

	fn runtime_api(&self) -> ApiRef<Self::Api> {
		self.runtime_api.clone().into()
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
