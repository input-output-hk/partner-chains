use sp_api::ApiError;

use super::*;

#[allow(deprecated)]
impl<C, Block> SidechainInfo<Block> for C
where
	C: Client<Block> + Send + Sync + 'static,
	C::Api: GetSidechainStatus<Block>,
	Block: BlockT,
	NumberFor<Block>: From<u32> + Into<u32>,
{
	type Error = ApiError;

	fn get_slot_of_block(
		&self,
		block_number: NumberFor<Block>,
	) -> Result<ScSlotNumber, Self::Error> {
		let api = self.runtime_api();
		let block_hash = self
			.hash(block_number)?
			.ok_or(ApiError::UnknownBlock(format!("Block Number {block_number} does not exist")))?;
		let sidechain_status = api.get_sidechain_status(block_hash)?;
		Ok(sidechain_status.slot)
	}

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
