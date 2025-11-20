use sp_api::ApiError;

use super::*;

#[allow(deprecated)]
impl<C, Block> FindSidechainBlock<Block> for C
where
	C: Client<Block> + Send + Sync + 'static,
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
	C::Api: GetSidechainStatus<Block>,
{
	type Error = ApiError;

	fn find_any_block_number_in_epoch(
		&self,
		epoch: ScEpochNumber,
	) -> Result<NumberFor<Block>, Self::Error> {
		let (left_block, right_block): (u32, u32) = (1u32, self.info().best_number.into());
		let range = left_block..right_block + 1;
		let f = |block: &u32| AnyBlockInEpoch { epoch }.compare_block((*block).into(), self);

		binary_search_by(range, f)
			.ok_or(ApiError::Application("Could not find block".to_string().into()))
			.map(|x| x.into())
	}

	/// Finds any block in the given epoch if it exists
	fn find_any_block_in_epoch(&self, epoch: ScEpochNumber) -> Result<Block::Hash, Self::Error> {
		let block_number = self.find_any_block_number_in_epoch(epoch)?;
		Ok(self
			.hash(block_number)?
			.expect("Block with given number exists, so its hash should exists as well"))
	}
}
