use sidechain_domain::*;
use sp_block_participation::inherent_data::BlockParticipationDataSource;

/// Mocked stake distribution data source that returns empty data
pub struct StakeDistributionDataSourceMock;

impl StakeDistributionDataSourceMock {
	/// Creates new data source
	pub fn new() -> Self {
		Self
	}
}

#[async_trait::async_trait]
impl BlockParticipationDataSource for StakeDistributionDataSourceMock {
	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		_epoch: McEpochNumber,
		_pool_hashes: &[MainchainKeyHash],
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		Ok(StakeDistribution::default())
	}
}
