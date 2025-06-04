use crate::Result;
use sidechain_domain::*;
use sp_timestamp::Timestamp;

/// Mock block data data source, for internal use by other mock data sources
///
/// This data source serves synthetic block data generated deterministically from the inputs.
pub struct BlockDataSourceMock {
	/// Duration of a mainchain epoch in milliseconds
	mc_epoch_duration_millis: u32,
}

impl BlockDataSourceMock {
	pub(crate) async fn get_latest_block_info(&self) -> Result<MainchainBlock> {
		Ok(self
			.get_latest_stable_block_for(Timestamp::new(BlockDataSourceMock::millis_now()))
			.await
			.unwrap()
			.unwrap())
	}

	pub(crate) async fn get_latest_stable_block_for(
		&self,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>> {
		let block_number = (reference_timestamp.as_millis() / 20000) as u32;
		let epoch = block_number / self.block_per_epoch();
		let mut hash_arr = [0u8; 32];
		hash_arr[..4].copy_from_slice(&block_number.to_be_bytes());
		Ok(Some(MainchainBlock {
			number: McBlockNumber(block_number),
			hash: McBlockHash(hash_arr),
			epoch: McEpochNumber(epoch),
			slot: McSlotNumber(block_number as u64),
			timestamp: reference_timestamp.as_millis(),
		}))
	}

	pub(crate) async fn get_stable_block_for(
		&self,
		_hash: McBlockHash,
		reference_timestamp: Timestamp,
	) -> Result<Option<MainchainBlock>> {
		self.get_latest_stable_block_for(reference_timestamp).await
	}

	pub(crate) async fn get_block_by_hash(
		&self,
		hash: McBlockHash,
	) -> Result<Option<MainchainBlock>> {
		// reverse of computation in `get_latest_stable_block_for`
		let block_number = u32::from_be_bytes(hash.0[..4].try_into().unwrap());
		let timestamp = block_number * 20000;
		let epoch = block_number / self.block_per_epoch();
		Ok(Some(MainchainBlock {
			number: McBlockNumber(block_number),
			hash,
			epoch: McEpochNumber(epoch),
			slot: McSlotNumber(epoch.into()),
			timestamp: timestamp.into(),
		}))
	}
}

impl BlockDataSourceMock {
	/// Creates new data source
	pub fn new(mc_epoch_duration_millis: u32) -> Self {
		Self { mc_epoch_duration_millis }
	}

	/// Creates new data source, reading configuration from environment
	pub fn new_from_env() -> Result<Self> {
		let mc_epoch_duration_millis: u32 =
			std::env::var("MC__EPOCH_DURATION_MILLIS")?.parse::<u32>()?;
		Ok(Self::new(mc_epoch_duration_millis))
	}

	fn block_per_epoch(&self) -> u32 {
		self.mc_epoch_duration_millis / 20000
	}

	fn millis_now() -> u64 {
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_millis() as u64
	}
}
