use async_trait::async_trait;
pub use main_chain_follower_api::block::*;
use main_chain_follower_api::common::*;
use main_chain_follower_api::*;
use sidechain_domain::*;
use std::env;

pub struct BlockDataSourceMock;

#[async_trait]
impl BlockDataSource for BlockDataSourceMock {
	async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
		Ok(self
			.get_latest_stable_block_for(Timestamp(BlockDataSourceMock::millis_now()))
			.await
			.unwrap()
			.unwrap())
	}

	async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>> {
		let block_number = (reference_timestamp.0 / 20000) as u32;
		let epoch = block_number / BlockDataSourceMock::blocks_per_epoch();
		let mut hash_arr = [0u8; 32];
		hash_arr[..4].copy_from_slice(&block_number.to_be_bytes());
		Ok(Some(MainchainBlock {
			number: McBlockNumber(block_number),
			hash: McBlockHash(hash_arr.to_vec()),
			epoch: McEpochNumber(epoch),
			slot: McSlotNumber(block_number as u64),
			timestamp: reference_timestamp.0,
		}))
	}

	async fn get_stable_block_for(
		&self,
		_hash: McBlockHash,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>> {
		self.get_latest_stable_block_for(reference_timestamp).await
	}
}

impl BlockDataSourceMock {
	fn millis_now() -> u64 {
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_millis() as u64
	}

	fn blocks_per_epoch() -> u32 {
		let val = env::var("MC__EPOCH_DURATION_MILLIS").unwrap();
		let millis: u32 = val.parse().unwrap();
		millis / 20000
	}
}
