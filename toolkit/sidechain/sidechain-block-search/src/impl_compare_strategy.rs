//! Module contains items that implement the trait `CompareStrategy`

use super::*;

impl<Block, BlockInfo> CompareStrategy<Block, BlockInfo> for AnyBlockInEpoch
where
	Block: BlockT,
	BlockInfo: SidechainInfo<Block>,
{
	type Error = BlockInfo::Error;

	fn compare_block(
		&self,
		block: NumberFor<Block>,
		block_info: &BlockInfo,
	) -> Result<Ordering, Self::Error> {
		let epoch_block = block_info.get_epoch_of_block(block)?;
		Ok(epoch_block.cmp(&self.epoch))
	}
}
