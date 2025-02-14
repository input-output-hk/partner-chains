use crate::db_model::{EpochNumber, StakePoolDelegationOutputRow};
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use derive_new::new;
use sidechain_domain::*;
use sp_stake_distribution::StakeDistributionDataSource;
use sqlx::PgPool;

#[cfg(test)]
mod tests;

#[derive(new)]
pub struct StakeDistributionDataSourceImpl {
	pub pool: PgPool,
	metrics_opt: Option<McFollowerMetrics>,
}

observed_async_trait!(
impl StakeDistributionDataSource for StakeDistributionDataSourceImpl {
	async fn get_stake_pool_delegation_distribution(
		&self,
		epoch: McEpochNumber,
	) -> Result<StakeDistribution, Box<dyn std::error::Error + Send + Sync>> {
		let rows = crate::db_model::get_stake_pool_delegations(&self.pool, EpochNumber::from(epoch)).await?;
		Ok(rows_to_distribution(rows))
	}
});

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
