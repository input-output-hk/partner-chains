use crate::db_model::{EpochNumber, StakePoolDelegationOutputRow};
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use lru::LruCache;
use sidechain_domain::*;
use sp_stake_distribution::StakeDistributionDataSource;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

pub struct StakeDistributionDataSourceImpl {
	pub pool: PgPool,
	metrics_opt: Option<McFollowerMetrics>,
	cache: Cache,
}

impl StakeDistributionDataSourceImpl {
	pub fn new(pool: PgPool, metrics_opt: Option<McFollowerMetrics>, cache_size: usize) -> Self {
		StakeDistributionDataSourceImpl { pool, metrics_opt, cache: Cache::new(cache_size) }
	}
}

observed_async_trait!(
impl StakeDistributionDataSource for StakeDistributionDataSourceImpl {
	async fn get_stake_pool_delegation_distribution(
		&self,
		epoch: McEpochNumber,
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		let rows =
			crate::db_model::get_stake_pool_delegations(&self.pool, EpochNumber::from(epoch))
				.await?;
		Ok(rows_to_distribution(rows))
	}

	async fn get_stake_pool_delegation_distribution_for_pool(
		&self,
		epoch: McEpochNumber,
		pool_hash: MainchainKeyHash,
	) -> Result<PoolDelegation, Box<dyn std::error::Error + Send + Sync>> {
		Ok(self
			.get_stake_pool_delegation_distribution_for_pools(epoch, vec![pool_hash])
			.await?
			.0
			.get(&pool_hash)
			.expect("infallible: result has to contain pool hash")
			.clone())
	}

	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		epoch: McEpochNumber,
		pool_hashes: Vec<MainchainKeyHash>,
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		let mut pool_hashes_to_query = Vec::<[u8; 28]>::new();
		let mut stake_distribution = BTreeMap::<MainchainKeyHash, PoolDelegation>::new();

		for pool_hash in pool_hashes {
			match self.cache.get_distribution_for_pool(epoch, pool_hash) {
				Some(pool_delegation) => {
					stake_distribution.insert(pool_hash, pool_delegation);
				},
				None => pool_hashes_to_query.push(pool_hash.0),
			}
		}
		let rows = crate::db_model::get_stake_pool_delegations_for_pools(
			&self.pool,
			EpochNumber::from(epoch),
			pool_hashes_to_query,
		)
		.await?;
		let mut queried_pool_delegations = rows_to_distribution(rows);
		self.cache.put_distribution_for_pools(epoch, queried_pool_delegations.clone());
		stake_distribution.append(&mut queried_pool_delegations.0);
		Ok(StakeDistribution(stake_distribution))
	}
}
);

fn rows_to_distribution(rows: Vec<StakePoolDelegationOutputRow>) -> StakeDistribution {
	let mut res = BTreeMap::<MainchainKeyHash, PoolDelegation>::new();
	for row in rows {
		match get_delegator_key(&row) {
			Ok(delegator_key) => {
				let pool = res.entry(MainchainKeyHash(row.pool_hash_raw)).or_default();
				pool.delegators
					.entry(delegator_key)
					.or_insert(DelegatorStakeAmount(row.epoch_stake_amount.0));
				pool.total_stake.0 += row.epoch_stake_amount.0;
			},
			Err(e) => {
				log::warn!("Failed to parse StakePoolDelegationOutputRow: {}", e)
			},
		}
	}
	StakeDistribution(res)
}

fn get_delegator_key(row: &StakePoolDelegationOutputRow) -> Result<DelegatorKey, String> {
	match &row.stake_address_hash_raw[..] {
		[0xe0 | 0xe1, rest @ ..] => Ok(DelegatorKey::StakeKeyHash(
			rest.try_into().expect("infallible: stake_address_hash_raw is 29 bytes"),
		)),
		[0xf0 | 0xf1, rest @ ..] => Ok(DelegatorKey::ScriptKeyHash {
			hash_raw: rest.try_into().expect("infallible: stake_address_hash_raw is 29 bytes"),
			script_hash: row
				.stake_address_script_hash
				.ok_or("stake_address_script_hash must be present for script keys")?,
		}),
		_ => {
			Err(format!("invalid stake address hash: {}", hex::encode(row.stake_address_hash_raw)))
		},
	}
}

type DistributionPerPoolCacheKey = (McEpochNumber, MainchainKeyHash);
struct Cache {
	distribution_per_pool_cache: Arc<Mutex<LruCache<DistributionPerPoolCacheKey, PoolDelegation>>>,
}

impl Cache {
	fn new(cache_size: usize) -> Self {
		Self {
			distribution_per_pool_cache: Arc::new(Mutex::new(LruCache::new(
				cache_size.try_into().unwrap(),
			))),
		}
	}

	fn get_distribution_for_pool(
		&self,
		epoch: McEpochNumber,
		pool_hash: MainchainKeyHash,
	) -> Option<PoolDelegation> {
		if let Ok(mut cache) = self.distribution_per_pool_cache.lock() {
			cache.get(&(epoch, pool_hash)).map(|e| e.clone())
		} else {
			None
		}
	}

	fn put_distribution_for_pools(
		&self,
		epoch: McEpochNumber,
		stake_distribution: StakeDistribution,
	) {
		if let Ok(mut cache) = self.distribution_per_pool_cache.lock() {
			for (pool_hash, pool_delegation) in stake_distribution.0 {
				cache.put((epoch, pool_hash), pool_delegation);
			}
		}
	}
}
