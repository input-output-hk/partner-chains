use hex_literal::hex;
use sidechain_domain::*;
use sp_stake_distribution::StakeDistributionDataSource;
use sqlx::PgPool;

use super::StakeDistributionDataSourceImpl;

#[sqlx::test(migrations = "./testdata/stake-distribution/migrations")]
async fn stake_distribution_works_for_no_epochs(pool: PgPool) {
	let result = make_source(pool)
		.get_stake_pool_delegation_distribution(Vec::new())
		.await
		.unwrap();

	assert!(result.is_empty());
}

#[sqlx::test(migrations = "./testdata/stake-distribution/migrations")]
async fn stake_distribution_works_for_single_epoch(pool: PgPool) {
	let result = make_source(pool)
		.get_stake_pool_delegation_distribution(vec![McEpochNumber(188)])
		.await
		.unwrap();

	let distribution_for_188 = &result.get(&McEpochNumber(188)).unwrap().0;
	let distribution_for_189 = &result.get(&McEpochNumber(189));

	assert!(distribution_for_188.contains_key(&stake_pool_key_hash_1()));
	assert!(distribution_for_188.contains_key(&stake_pool_key_hash_2()));
	assert!(distribution_for_189.is_none());
	assert_eq!(distribution_for_188.get(&stake_pool_key_hash_1()).unwrap(), &pool_delegation_1());
	assert_eq!(distribution_for_188.get(&stake_pool_key_hash_2()).unwrap(), &pool_delegation_2());
}

#[sqlx::test(migrations = "./testdata/stake-distribution/migrations")]
async fn stake_distribution_works_for_multiple_epochs(pool: PgPool) {
	let result = make_source(pool)
		.get_stake_pool_delegation_distribution(vec![McEpochNumber(188), McEpochNumber(189)])
		.await
		.unwrap();

	let distribution_for_188 = &result.get(&McEpochNumber(188)).unwrap().0;
	let distribution_for_189 = &result.get(&McEpochNumber(189)).unwrap().0;

	assert!(distribution_for_188.contains_key(&stake_pool_key_hash_1()));
	assert!(distribution_for_188.contains_key(&stake_pool_key_hash_2()));
	assert!(distribution_for_189.contains_key(&stake_pool_key_hash_1()));
	assert!(distribution_for_189.contains_key(&stake_pool_key_hash_2()));
	assert_eq!(distribution_for_188.get(&stake_pool_key_hash_1()).unwrap(), &pool_delegation_1());
	assert_eq!(distribution_for_188.get(&stake_pool_key_hash_2()).unwrap(), &pool_delegation_2());
}

fn make_source(pool: PgPool) -> StakeDistributionDataSourceImpl {
	StakeDistributionDataSourceImpl::new(pool, None)
}

fn stake_pool_key_hash_1() -> StakePoolKeyHash {
	StakePoolKeyHash(hex!("38f4a58aaf3fec84f3410520c70ad75321fb651ada7ca026373ce486"))
}
fn stake_pool_key_hash_2() -> StakePoolKeyHash {
	StakePoolKeyHash(hex!("d5cfc42cf67f6b637688d19fa50a4342658f63370b9e2c9e3eaf4dfe"))
}

fn delegator_address_hash_1() -> DelegatorAddressHash {
	DelegatorAddressHash(hex!("e033916328baa83c42dbdcde825122ccf024ca3599c19ca6fb1697dc93"))
}
fn delegator_address_hash_2() -> DelegatorAddressHash {
	DelegatorAddressHash(hex!("e0aa898fce3be344c6be2d86fe1c5918675c9b0672cda8ab809d262824"))
}
fn delegator_address_hash_3() -> DelegatorAddressHash {
	DelegatorAddressHash(hex!("e0c55157ae1b08643719584c4972132ed210c64b02da80004cbd9b8c7f"))
}
fn delegator_address_hash_4() -> DelegatorAddressHash {
	DelegatorAddressHash(hex!("e0ad148225d7fb809f74a07d2dbc2eef91617f603bfb731e634bf8a1a9"))
}
fn delegator_address_hash_5() -> DelegatorAddressHash {
	DelegatorAddressHash(hex!("e0b96fcef3b9351af6834bd850e3a97859d7bd5b729d24bf3646aeaccf"))
}
fn delegator_address_hash_6() -> DelegatorAddressHash {
	DelegatorAddressHash(hex!("e0ba149e2e2379097e65f0c03f2733d3103151e7f100d36dfdb01a0b22"))
}
fn script_hash_1() -> DelegatorScriptHash {
	DelegatorScriptHash(hex!("ec6586f39718287300c62842ddfff37a541c46acffb379744ba1063a"))
}

fn pool_delegation_1() -> PoolDelegation {
	PoolDelegation {
		total_stake: TotalStakeAmount(5001995651486),
		delegators: [
			(
				DelegationKey { delegator_address: delegator_address_hash_1(), script_hash: None },
				DelegatorStakeAmount(5000000000000),
			),
			(
				DelegationKey { delegator_address: delegator_address_hash_2(), script_hash: None },
				DelegatorStakeAmount(997825743),
			),
			(
				DelegationKey { delegator_address: delegator_address_hash_3(), script_hash: None },
				DelegatorStakeAmount(997825743),
			),
		]
		.into(),
	}
}
fn pool_delegation_2() -> PoolDelegation {
	PoolDelegation {
		total_stake: TotalStakeAmount(1001995478725),
		delegators: [
			(
				DelegationKey { delegator_address: delegator_address_hash_4(), script_hash: None },
				DelegatorStakeAmount(997825743),
			),
			(
				DelegationKey {
					delegator_address: delegator_address_hash_5(),
					script_hash: Some(script_hash_1()),
				},
				DelegatorStakeAmount(1000000000000),
			),
			(
				DelegationKey { delegator_address: delegator_address_hash_6(), script_hash: None },
				DelegatorStakeAmount(997652982),
			),
		]
		.into(),
	}
}
