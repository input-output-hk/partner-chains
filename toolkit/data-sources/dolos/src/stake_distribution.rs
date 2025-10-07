use sidechain_domain::*;
use sp_block_participation::inherent_data::BlockParticipationDataSource;

pub struct StakeDistributionDataSourceImpl;

impl StakeDistributionDataSourceImpl {
	pub fn new() -> Self {
		Self {}
	}
}

#[async_trait::async_trait]
impl BlockParticipationDataSource for StakeDistributionDataSourceImpl {
	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		_epoch: McEpochNumber,
		_pool_hashes: &[MainchainKeyHash],
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		Err("not implemented".into())
	}
}
