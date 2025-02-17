#![cfg_attr(not(feature = "std"), no_std)]

use sidechain_domain::*;

#[async_trait::async_trait]
pub trait StakeDistributionDataSource {
	/// Retrieves stake pool delegation distribution for provided epoch and pool
	async fn get_stake_pool_delegation_distribution_for_pool(
		&self,
		epoch: McEpochNumber,
		pool_hash: MainchainKeyHash,
	) -> Result<PoolDelegation, Box<dyn std::error::Error + Send + Sync>>;

	/// Retrieves stake pool delegation distribution for provided epoch and pools
	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		epoch: McEpochNumber,
		pool_hashes: &[MainchainKeyHash],
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>>;
}
