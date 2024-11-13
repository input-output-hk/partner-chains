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

impl<Block, BlockInfo> CompareStrategy<Block, BlockInfo> for LastBlockInEpoch
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
		let ordering = match epoch_block.cmp(&self.epoch) {
			Ordering::Equal => {
				match block_info.get_epoch_of_block(block + 1u32.into())?.cmp(&self.epoch) {
					Ordering::Greater => Ordering::Equal,
					_ => Ordering::Less,
				}
			},
			ordering => ordering,
		};
		Ok(ordering)
	}
}

impl<Block, BlockInfo> CompareStrategy<Block, BlockInfo> for AnyBlockInSlotRange
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
		let slot_of_block = block_info.get_slot_of_block(block)?;
		let ordering = if slot_of_block.0 < self.slot_range.start.0 {
			Ordering::Less
		} else if slot_of_block.0 >= self.slot_range.end.0 {
			Ordering::Greater
		} else {
			Ordering::Equal
		};

		Ok(ordering)
	}
}

impl<Block, BlockInfo> CompareStrategy<Block, BlockInfo> for LatestBlockInSlotRange<Block>
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
		let current_block_ordering = AnyBlockInSlotRange { slot_range: self.slot_range.clone() }
			.compare_block(block, block_info)?;
		let ordering = match current_block_ordering {
			Ordering::Equal => {
				if block < self.latest_block {
					let next_block_ordering =
						AnyBlockInSlotRange { slot_range: self.slot_range.clone() }
							.compare_block(block + 1u32.into(), block_info)?;
					match next_block_ordering {
						Ordering::Greater => Ordering::Equal,
						_ => Ordering::Less,
					}
				} else {
					Ordering::Equal
				}
			},
			ordering => ordering,
		};
		Ok(ordering)
	}
}
