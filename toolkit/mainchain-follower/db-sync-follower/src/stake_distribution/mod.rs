use crate::db_model::{EpochNumber, StakePoolDelegationOutputRow};
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use derive_new::new;
use itertools::Itertools;
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
		epochs: Vec<McEpochNumber>,
	) -> Result<BTreeMap<McEpochNumber, StakeDistribution>, Box<dyn std::error::Error + Send + Sync>> {
		let epochs = epochs.into_iter().map(EpochNumber::from).collect_vec();
		let rows = crate::db_model::get_stake_pool_delegations(&self.pool, epochs).await?;
		Ok(rows_to_distribution(rows))
	}
});

fn rows_to_distribution(
	rows: Vec<StakePoolDelegationOutputRow>,
) -> BTreeMap<McEpochNumber, StakeDistribution> {
	let mut res = BTreeMap::<McEpochNumber, StakeDistribution>::new();
	for row in rows {
		let per_epoch_distro = res.entry(McEpochNumber(row.epoch_number.0)).or_default();
		let pool = per_epoch_distro.0.entry(MainchainKeyHash(row.pool_hash_raw)).or_default();
		let delegator_key = match &row.stake_address_hash_raw[..] {
			[0xe0, rest @ ..] => DelegatorKey::StakeKeyHash(
				rest.try_into().expect("stake_address_hash_raw is 29 bytes"),
			),
			[0xf0, rest @ ..] => DelegatorKey::ScriptKeyHash {
				hash_raw: rest.try_into().expect("stake_address_hash_raw is 29 bytes"),
				script_hash: row
					.stake_address_script_hash
					.expect("stake_address_script_hash must be present for script keys"),
			},
			_ => panic!("invalid starting address byte on stake_address_hash_raw"),
		};
		pool.delegators
			.entry(delegator_key)
			.or_insert(DelegatorStakeAmount(row.epoch_stake_amount.0));
		pool.total_stake.0 += row.epoch_stake_amount.0;
	}
	res
}
