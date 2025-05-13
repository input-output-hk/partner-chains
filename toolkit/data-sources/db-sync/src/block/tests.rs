use crate::block::{BlockDataSourceImpl, BlocksCache, MainchainBlock};
use chrono::{NaiveDateTime, TimeDelta};
use hex_literal::hex;
use pallet_sidechain_rpc::SidechainRpcDataSource;
use sidechain_domain::mainchain_epoch::{Duration, MainchainEpochConfig, Timestamp};
use sidechain_domain::{McBlockHash, McBlockNumber, McEpochNumber, McSlotNumber};
use sidechain_mc_hash::McHashDataSource;
use sqlx::PgPool;
use std::str::FromStr;

const BLOCK_4_TS_MILLIS: u64 = 1650561570000;
const BLOCK_5_TS_MILLIS: u64 = 1650562570000;

#[sqlx::test(migrations = "./testdata/migrations")]
async fn get_latest_block_info(pool: PgPool) {
	let irrelevant_security_parameter = 1000;
	let source = mk_datasource(pool, irrelevant_security_parameter);
	let expected = MainchainBlock {
		number: McBlockNumber(5),
		hash: McBlockHash(hex!("EBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1")),
		epoch: McEpochNumber(193),
		slot: McSlotNumber(193500),
		timestamp: NaiveDateTime::from_str("2022-04-21T17:36:10")
			.unwrap()
			.and_utc()
			.timestamp()
			.try_into()
			.unwrap(),
	};
	let block = source.get_latest_block_info().await.unwrap();
	assert_eq!(block, expected)
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_latest_stable_block(pool: PgPool) {
	let security_parameter = 2;
	let source = mk_datasource(pool, security_parameter);
	// The latest block at given timestamp is 4, so the expected block is 2 (security parameter is 2)
	let expected = block_2();
	let exact_ts = BLOCK_4_TS_MILLIS.into();
	let block = source.get_latest_stable_block_for(exact_ts).await.unwrap();
	assert_eq!(block, Some(expected.clone()));
	let greater_ts = (BLOCK_4_TS_MILLIS + 1).into();
	let block = source.get_latest_stable_block_for(greater_ts).await.unwrap();
	assert_eq!(block, Some(expected))
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_latest_stable_block_at_filters_out_by_max_slot_boundary(pool: PgPool) {
	let security_parameter = 2;
	let slots_distance_between_blocks = block_4().slot.0 - block_2().slot.0;
	let source = BlockDataSourceImpl {
		max_slot_boundary_as_seconds: TimeDelta::seconds(
			(slots_distance_between_blocks - 1) as i64,
		),
		..mk_datasource(pool, security_parameter)
	};
	let ts = BLOCK_4_TS_MILLIS.into();
	let block = source.get_latest_stable_block_for(ts).await.unwrap();
	assert_eq!(block, None)
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_stable_block_at(pool: PgPool) {
	let security_parameter = 2;
	let source = mk_datasource(pool, security_parameter);
	let exact_ts = BLOCK_4_TS_MILLIS.into();
	let block = source.get_stable_block_for(block_2().hash, exact_ts).await.unwrap();
	assert_eq!(block, Some(block_2()));
	let greater_ts = (BLOCK_4_TS_MILLIS + 1).into();
	let block = source.get_stable_block_for(block_2().hash, greater_ts).await.unwrap();
	assert_eq!(block, Some(block_2()));
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_stable_block_at_returns_block_that_dont_have_k_blocks_on_them_at_given_timestamp(
	pool: PgPool,
) {
	let security_parameter = 3;
	let source = mk_datasource(pool, security_parameter);
	// Does not matter, that at block 4 timestamp, there were only two blocks on block number 2.
	// It only matters, that block 2 hash is correct and it's time is correct.
	let block = source
		.get_stable_block_for(block_2().hash, BLOCK_4_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(block, Some(block_2()));
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_stable_block_at_filters_out_by_min_slots_boundary(pool: PgPool) {
	let security_parameter = 3;
	let slots_distance_between_blocks = block_4().slot.0 - block_2().slot.0;
	let source = BlockDataSourceImpl {
		min_slot_boundary_as_seconds: TimeDelta::seconds(
			(slots_distance_between_blocks + 1) as i64,
		),
		..mk_datasource(pool, security_parameter)
	};
	// At the time of asking, block 2 is stable, because there are 3 blocks on top of it,
	// however it is too young according to the given timestamp
	let block = source
		.get_stable_block_for(block_2().hash, BLOCK_4_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(block, None);
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_stable_block_at_filters_out_by_max_slots_boundary(pool: PgPool) {
	let security_parameter = 2;
	let slots_distance_between_blocks = block_4().slot.0 - block_2().slot.0;
	let source = BlockDataSourceImpl {
		max_slot_boundary_as_seconds: TimeDelta::seconds(
			(slots_distance_between_blocks - 1) as i64,
		),
		..mk_datasource(pool, security_parameter)
	};
	// At the time of asking, block 2 is stable, because there are 3 blocks on top of it,
	// however it is too young according to the given timestamp
	let block = source
		.get_stable_block_for(block_2().hash, BLOCK_4_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(block, None);
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_stable_block_info_by_hash_for_unknown_hash(pool: PgPool) {
	let source = mk_datasource(pool, 2);
	let unknown_hash =
		McBlockHash(hex!("0000D7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1"));
	let result = source
		.get_stable_block_for(unknown_hash, BLOCK_4_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(result, None)
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_latest_stable_block_with_stability_margin(pool: PgPool) {
	let security_parameter = 2;
	let stability_margin = 1;
	let min_slot_boundary_as_seconds = 2000u64;
	let max_slot_boundary_as_seconds = 5000u64;
	let source = BlockDataSourceImpl {
		block_stability_margin: stability_margin,
		min_slot_boundary_as_seconds: TimeDelta::seconds(min_slot_boundary_as_seconds as i64),
		max_slot_boundary_as_seconds: TimeDelta::seconds(max_slot_boundary_as_seconds as i64),
		..mk_datasource(pool, security_parameter)
	};
	let min_ts_for_block_2 = (block_2().timestamp + min_slot_boundary_as_seconds) * 1000;
	let block = source.get_latest_stable_block_for(min_ts_for_block_2.into()).await.unwrap();
	assert_eq!(block, Some(block_2()));
	let ts_too_low_for_block_2 = min_ts_for_block_2 - 1;
	let block = source.get_latest_stable_block_for(ts_too_low_for_block_2.into()).await.unwrap();
	assert_eq!(block, Some(block_1()));
	let max_ts_for_block_2 = (block_2().timestamp + max_slot_boundary_as_seconds) * 1000;
	let block = source.get_latest_stable_block_for(max_ts_for_block_2.into()).await.unwrap();
	assert_eq!(block, Some(block_2()));
	let ts_too_high_for_block_2 = max_ts_for_block_2 + 1;
	let block = source
		.get_latest_stable_block_for(ts_too_high_for_block_2.into())
		.await
		.unwrap();
	assert_eq!(block, None);
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_latest_stable_block_with_stability_margin_2(pool: PgPool) {
	let security_parameter = 2;
	let stability_margin = 0;
	let source = BlockDataSourceImpl {
		block_stability_margin: stability_margin,
		..mk_datasource(pool.clone(), security_parameter)
	};
	// With block_stability_margin set to 0 only security_parameter is considered,
	// so the expected block is 2, because block 3 timestamp is less than min_slot_boundary_as_seconds from block 4 ts.
	let ts = BLOCK_4_TS_MILLIS.into();
	let block = source.get_latest_stable_block_for(ts).await.unwrap();
	assert_eq!(block, Some(block_2()));

	let security_parameter = 2;
	let stability_margin = 100;
	let source = BlockDataSourceImpl {
		block_stability_margin: stability_margin,
		..mk_datasource(pool, security_parameter)
	};
	// when latest_stable_block_number - (block_stability_margin + security_parameter) < 0
	// then the expected block is the genesis block
	let block = source.get_latest_stable_block_for(ts).await.unwrap();
	assert_eq!(block, Some(block_0()));
}

#[sqlx::test(migrations = "./testdata/migrations")]
async fn test_get_stable_block_caching(pool: PgPool) {
	fn dummy_hash(n: u8) -> McBlockHash {
		McBlockHash([n; 32])
	}

	async fn update_block_hash_in_db(pool: &PgPool, n: u8) {
		let sql = "UPDATE block SET hash = $1 WHERE block_no = $2";
		_ = sqlx::query(sql)
			.bind(dummy_hash(n).0)
			.bind(n as i32)
			.execute(pool)
			.await
			.unwrap();
	}

	let security_parameter = 2;
	let source =
		BlockDataSourceImpl { cache_size: 2, ..mk_datasource(pool.clone(), security_parameter) };
	// Scenario:
	// - query for block 0 (by hash)
	// - blocks 0, 1, 2 gets to "by hash" cache
	// - blocks 3, 4, 5 gets to "by timestamp" cache
	// - update hashes in the db rows for blocks 0, 1, 2 and 3
	// - query for block 0, 1, 2, 3 (by hash)
	// - blocks 0, 1, 2 should be returned by the original hash
	// - block 3 should be found only by the updated hash

	let result = source
		.get_stable_block_for(block_0().hash, BLOCK_5_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(result, Some(block_0()));

	update_block_hash_in_db(&pool, 0).await;
	update_block_hash_in_db(&pool, 1).await;
	update_block_hash_in_db(&pool, 2).await;
	update_block_hash_in_db(&pool, 3).await;

	let result = source
		.get_stable_block_for(block_1().hash, BLOCK_5_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(result, Some(block_1()));

	let result = source
		.get_stable_block_for(block_2().hash, BLOCK_5_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(result, Some(block_2()));

	let result = source
		.get_stable_block_for(block_3().hash, BLOCK_5_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(result, None);

	let result = source
		.get_stable_block_for(dummy_hash(3), BLOCK_5_TS_MILLIS.into())
		.await
		.unwrap();
	assert_eq!(result, Some(MainchainBlock { hash: dummy_hash(3), ..block_3() }))
}

fn mk_datasource(pool: PgPool, security_parameter: u32) -> BlockDataSourceImpl {
	BlockDataSourceImpl {
		pool,
		security_parameter,
		min_slot_boundary_as_seconds: TimeDelta::seconds(1667),
		max_slot_boundary_as_seconds: TimeDelta::seconds(5000),
		mainchain_epoch_config: mainchain_epoch_config(),
		block_stability_margin: 0,
		cache_size: 100,
		stable_blocks_cache: BlocksCache::new_arc_mutex(),
	}
}

fn mainchain_epoch_config() -> MainchainEpochConfig {
	// Matches data of block 0 from 5_insert_blocks.sql
	MainchainEpochConfig {
		first_epoch_timestamp_millis: Timestamp::from_unix_millis(1650558070000),
		epoch_duration_millis: Duration::from_millis(1000 * 1000),
		first_epoch_number: 189,
		first_slot_number: 189000,
		slot_duration_millis: Duration::from_millis(1000),
	}
}

fn block_0() -> MainchainBlock {
	MainchainBlock {
		number: McBlockNumber(0),
		hash: McBlockHash(hex!("0BEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1")),
		epoch: McEpochNumber(189),
		slot: McSlotNumber(189410),
		timestamp: 1650558480, // 2022-04-21T16:28:00Z
	}
}

fn block_1() -> MainchainBlock {
	MainchainBlock {
		number: McBlockNumber(1),
		hash: McBlockHash(hex!("ABEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1")),
		epoch: McEpochNumber(190),
		slot: McSlotNumber(190400),
		timestamp: 1650559470, // 2022-04-21T16:44:30Z
	}
}

fn block_2() -> MainchainBlock {
	MainchainBlock {
		number: McBlockNumber(2),
		hash: McBlockHash(hex!("BBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1")),
		epoch: McEpochNumber(190),
		slot: McSlotNumber(190500),
		timestamp: 1650559570, // 2022-04-23T16:46:10Z
	}
}

fn block_3() -> MainchainBlock {
	MainchainBlock {
		number: McBlockNumber(3),
		hash: McBlockHash(hex!("CBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1")),
		epoch: McEpochNumber(191),
		slot: McSlotNumber(191500),
		timestamp: 1650560570, // 2022-04-21T17:02:50Z
	}
}

fn block_4() -> MainchainBlock {
	MainchainBlock {
		number: McBlockNumber(4),
		hash: McBlockHash(hex!("DBEED7FB0067F14D6F6436C7F7DEDB27CE3CEB4D2D18FF249D43B22D86FAE3F1")),
		epoch: McEpochNumber(192),
		slot: McSlotNumber(192500),
		timestamp: 1650561570, // 2022-04-25T17:19:30Z
	}
}
