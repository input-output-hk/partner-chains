use sidechain_domain::ScEpochNumber;
use sp_api::ApiError;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::NumberFor;
#[allow(deprecated)]
use sp_sidechain::GetSidechainStatus;
use std::cmp::Ordering;

/// Runtime API client used by the block queries in this crate
pub trait Client<Block: BlockT>: HeaderBackend<Block> + ProvideRuntimeApi<Block> {}

impl<C: HeaderBackend<Block> + ProvideRuntimeApi<Block>, Block: BlockT> Client<Block> for C {}

/// Interface for retrieving information about slot and epoch of Partner Chain blocks
pub trait SidechainInfo<Block: BlockT>: Client<Block> {
	/// Error type
	type Error: std::error::Error;

	/// Finds the Partner Chain eopch number for a given block number
	fn get_epoch_of_block(
		&self,
		block_number: NumberFor<Block>,
	) -> Result<ScEpochNumber, Self::Error>;
}

#[allow(deprecated)]
impl<C, Block> SidechainInfo<Block> for C
where
	C: Client<Block> + Send + Sync + 'static,
	C::Api: GetSidechainStatus<Block>,
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
{
	type Error = ApiError;

	fn get_epoch_of_block(
		&self,
		block_number: NumberFor<Block>,
	) -> Result<ScEpochNumber, Self::Error> {
		let api = self.runtime_api();
		let block_hash = self
			.hash(block_number)?
			.ok_or(ApiError::UnknownBlock(format!("Block Number {block_number} does not exist")))?;
		let sidechain_status = api.get_sidechain_status(block_hash)?;
		Ok(sidechain_status.epoch)
	}
}

/// Runtime client capable of finding Partner Chain blocks via binary search
pub trait FindSidechainBlock<Block: BlockT>: Client<Block> + Sized {
	/// Error type
	type Error: std::error::Error;

	/// Finds any block in the given epoch if it exists
	fn find_any_block_in_epoch(&self, epoch: ScEpochNumber) -> Result<Block::Hash, Self::Error>;
}

#[allow(deprecated)]
impl<C, Block> FindSidechainBlock<Block> for C
where
	C: Client<Block> + Send + Sync + 'static,
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
	C::Api: GetSidechainStatus<Block>,
{
	type Error = ApiError;

	/// Finds any block in the given epoch if it exists
	fn find_any_block_in_epoch(&self, epoch: ScEpochNumber) -> Result<Block::Hash, Self::Error> {
		let mut left = 1u32;
		let mut right: u32 = self.info().best_number.into();

		while left <= right {
			let middle = (left + right) / 2;
			let block_epoch = self.get_epoch_of_block(middle.into())?;

			match block_epoch.cmp(&epoch) {
				Ordering::Less => left = middle + 1,
				Ordering::Greater => right = middle - 1,
				Ordering::Equal => {
					return Ok(self.hash(middle.into())?.expect(
						"Block with given number exists, so its hash should exists as well",
					));
				},
			}
		}

		return Err(ApiError::Application("Could not find block".to_string().into()));
	}
}
