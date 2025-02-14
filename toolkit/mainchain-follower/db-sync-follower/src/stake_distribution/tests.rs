use hex_literal::hex;
use sidechain_domain::*;
use sp_stake_distribution::StakeDistributionDataSource;
use sqlx::PgPool;

use super::StakeDistributionDataSourceImpl;

#[sqlx::test(migrations = "./testdata/stake-distribution/migrations")]
async fn stake_distribution_works(pool: PgPool) {
	let epoch = McEpochNumber(188);
	let distribution =
		make_source(pool).get_stake_pool_delegation_distribution(epoch).await.unwrap().0;

	assert_eq!(distribution.get(&stake_pool_key_hash_1()).unwrap(), &pool_delegation_1());
	assert_eq!(distribution.get(&stake_pool_key_hash_2()).unwrap(), &pool_delegation_2());
}

fn make_source(pool: PgPool) -> StakeDistributionDataSourceImpl {
	StakeDistributionDataSourceImpl::new(pool, None)
}

fn stake_pool_key_hash_1() -> MainchainKeyHash {
	MainchainKeyHash(hex!("38f4a58aaf3fec84f3410520c70ad75321fb651ada7ca026373ce486"))
}
fn stake_pool_key_hash_2() -> MainchainKeyHash {
	MainchainKeyHash(hex!("d5cfc42cf67f6b637688d19fa50a4342658f63370b9e2c9e3eaf4dfe"))
}

fn delegator_address_hash_1() -> [u8; 28] {
	hex!("33916328baa83c42dbdcde825122ccf024ca3599c19ca6fb1697dc93")
}
fn delegator_address_hash_2() -> [u8; 28] {
	hex!("aa898fce3be344c6be2d86fe1c5918675c9b0672cda8ab809d262824")
}
fn delegator_address_hash_3() -> [u8; 28] {
	hex!("49b16fb356be9e46778478f2c9601a24fa16c88b2a97681d5af06d01")
}
fn delegator_address_hash_4() -> [u8; 28] {
	hex!("ba149e2e2379097e65f0c03f2733d3103151e7f100d36dfdb01a0b22")
}
fn script_hash_1() -> [u8; 28] {
	hex!("49b16fb356be9e46778478f2c9601a24fa16c88b2a97681d5af06d01")
}

fn pool_delegation_1() -> PoolDelegation {
	PoolDelegation {
		total_stake: StakeDelegation(5000997825743),
		delegators: [
			(
				DelegatorKey::ScriptKeyHash {
					hash_raw: delegator_address_hash_1(),
					script_hash: script_hash_1(),
				},
				DelegatorStakeAmount(5000000000000),
			),
			(
				DelegatorKey::StakeKeyHash(delegator_address_hash_2()),
				DelegatorStakeAmount(997825743),
			),
		]
		.into(),
	}
}
fn pool_delegation_2() -> PoolDelegation {
	PoolDelegation {
		total_stake: StakeDelegation(1000997652982),
		delegators: [
			(
				DelegatorKey::ScriptKeyHash {
					hash_raw: delegator_address_hash_3(),
					script_hash: script_hash_1(),
				},
				DelegatorStakeAmount(1000000000000),
			),
			(
				DelegatorKey::StakeKeyHash(delegator_address_hash_4()),
				DelegatorStakeAmount(997652982),
			),
		]
		.into(),
	}
}
