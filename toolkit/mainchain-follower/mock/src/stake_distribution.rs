use sidechain_domain::*;
use sp_stake_distribution::StakeDistributionDataSource;

pub struct StakeDistributionDataSourceMock;

impl StakeDistributionDataSourceMock {
	pub fn new() -> Self {
		Self
	}
}

#[async_trait::async_trait]
impl StakeDistributionDataSource for StakeDistributionDataSourceMock {
	async fn get_stake_pool_delegation_distribution_for_pool(
		&self,
		_epoch: McEpochNumber,
		_pool_hash: MainchainKeyHash,
	) -> Result<PoolDelegation, Box<dyn std::error::Error + Send + Sync>> {
		Ok(PoolDelegation::default())
	}

	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		_epoch: McEpochNumber,
		_pool_hashes: &[MainchainKeyHash],
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		Ok(StakeDistribution::default())
	}
}
