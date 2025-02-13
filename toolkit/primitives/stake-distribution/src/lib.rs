#![cfg_attr(not(feature = "std"), no_std)]

use sidechain_domain::*;

#[async_trait::async_trait]
pub trait StakeDistributionDataSource {
	/// Retrieves stake pool delegation distribution for provided epochs
	async fn get_stake_pool_delegation_distribution(
		&self,
		epoch: McEpochNumber,
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>>;
}
