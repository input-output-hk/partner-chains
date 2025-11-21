use crate::{
	Result,
	client::{MiniBFClient, api::MiniBFApi},
};
use blockfrost_openapi::models::epoch_stake_pool_content_inner::EpochStakePoolContentInner;
use futures::StreamExt;
use sidechain_domain::*;
use sp_block_participation::inherent_data::BlockParticipationDataSource;

pub struct StakeDistributionDataSourceImpl {
	client: MiniBFClient,
}

impl StakeDistributionDataSourceImpl {
	pub fn new(client: MiniBFClient) -> Self {
		Self { client }
	}
}

#[async_trait::async_trait]
impl BlockParticipationDataSource for StakeDistributionDataSourceImpl {
	async fn get_stake_pool_delegation_distribution_for_pools(
		&self,
		epoch_number: McEpochNumber,
		pool_hashes: &[MainchainKeyHash],
	) -> Result<StakeDistribution> {
		let pool_futures = futures::stream::iter(pool_hashes)
			.map(|pool_id| async {
				self.client
					.epochs_stakes_by_pool(epoch_number, *pool_id)
					.await
					.map(|ss| ss.iter().map(|s| (*pool_id, s.clone())).collect::<Vec<_>>())
			})
			.collect::<Vec<_>>()
			.await;
		let pools = futures::future::try_join_all(pool_futures)
			.await?
			.into_iter()
			.flatten()
			.collect::<Vec<_>>();
		Ok(rows_to_distribution(pools))
	}
}

fn rows_to_distribution(
	rows: Vec<(sidechain_domain::MainchainKeyHash, EpochStakePoolContentInner)>,
) -> StakeDistribution {
	let mut res = BTreeMap::<MainchainKeyHash, PoolDelegation>::new();
	for (pool_id, stake) in rows {
		match get_delegator_key(&stake) {
			Ok(delegator_key) => {
				let pool = res.entry(pool_id).or_default();
				let stake_amount = stake.amount.parse().expect("valid stake amount");
				pool.delegators
					.entry(delegator_key)
					.or_insert(DelegatorStakeAmount(stake_amount));
				pool.total_stake.0 += stake_amount;
			},
			Err(e) => {
				log::warn!("Failed to parse EpochStakePoolContentInner: {}", e)
			},
		}
	}
	StakeDistribution(res)
}

fn get_delegator_key(row: &EpochStakePoolContentInner) -> Result<DelegatorKey> {
	let (_, stake_address_hash_raw) = bech32::decode(&row.stake_address)?;
	match &stake_address_hash_raw[..] {
		[0xe0 | 0xe1, rest @ ..] => Ok(DelegatorKey::StakeKeyHash(
			rest.try_into().expect("infallible: stake_address_hash_raw is 29 bytes"),
		)),
		[0xf0 | 0xf1, rest @ ..] => Ok(DelegatorKey::ScriptKeyHash {
			hash_raw: rest.try_into().expect("infallible: stake_address_hash_raw is 29 bytes"),
			script_hash: [0; 28], // TODO how to get this?
		}),
		_ => Err(format!("invalid stake address hash: {}", row.stake_address).into()),
	}
}
