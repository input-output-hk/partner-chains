use sp_api::ApiError;

use super::*;

impl<C, Block, CS> FindSidechainBlock<Block, CS> for C
where
	C: Client<Block> + Send + Sync + 'static,
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
	CS: CompareStrategy<Block, Self>,
{
	type Error = ApiError;

	fn find_block_number(&self, compare_strategy: CS) -> Result<NumberFor<Block>, Self::Error> {
		let (left_block, right_block): (u32, u32) = (1u32, self.info().best_number.into());
		let range = left_block..right_block + 1;
		let f = |block: &u32| compare_strategy.compare_block((*block).into(), self);

		binary_search_by(range, f)
			.ok_or(ApiError::Application("Could not find block".to_string().into()))
			.map(|x| x.into())
	}

	fn find_block(&self, compare_strategy: CS) -> Result<Block::Hash, Self::Error> {
		let block_number = self.find_block_number(compare_strategy)?;
		Ok(self
			.hash(block_number)?
			.expect("Block with given number exists, so its hash should exists as well"))
	}
}
