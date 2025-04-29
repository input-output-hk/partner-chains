use super::{Cache, GovernedMapDataSourceCachedImpl, GovernedMapDataSourceImpl};
use crate::metrics::mock::test_metrics;
use hex_literal::hex;
use pretty_assertions::assert_eq;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio_test::assert_err;

// tx1: inserts key2
// tx2: inserts key1, an invalid datum, and duplicate key2
const BLOCK_1: McBlockHash =
	McBlockHash(hex!("b702000000000000000000000000000000000000000000000000000000000002"));
// tx3: deletes key1
const BLOCK_4: McBlockHash =
	McBlockHash(hex!("b702000000000000000000000000000000000000000000000000000000000005"));
// tx4: inserts key3
const BLOCK_6: McBlockHash =
	McBlockHash(hex!("b702000000000000000000000000000000000000000000000000000000000007"));
// tx5: updates key3
const BLOCK_7: McBlockHash =
	McBlockHash(hex!("b702000000000000000000000000000000000000000000000000000000000008"));

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_fails_on_wrong_block_hash(pool: PgPool) {
	let source = make_source(pool);
	let mc_block =
		McBlockHash(hex!("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
	let result = source.get_mapping_changes(None, mc_block, scripts()).await;
	assert_err!(result);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_cached_governed_map_fails_on_wrong_block_hash(pool: PgPool) {
	let source = make_cached_source(pool);
	let mc_block =
		McBlockHash(hex!("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
	let result = source.get_mapping_changes(None, mc_block, scripts()).await;
	assert_err!(result);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_insert(pool: PgPool) {
	let source = make_source(pool);
	let mut result = source.get_mapping_changes(None, BLOCK_1, scripts()).await.unwrap();
	result.sort();
	let expected = vec![
		(
			"key1".to_owned(),
			Some(ByteString::from(hex!("11111111111111111111111111111111").to_vec())),
		),
		(
			"key2".to_owned(),
			Some(ByteString::from(hex!("22222222222222222222222222222222").to_vec())),
		),
	];
	assert_eq!(result, expected);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_cached_governed_map_insert(pool: PgPool) {
	let source = make_cached_source(pool);
	let mut result = source.get_mapping_changes(None, BLOCK_1, scripts()).await.unwrap();
	result.sort();
	let expected = vec![
		(
			"key1".to_owned(),
			Some(ByteString::from(hex!("11111111111111111111111111111111").to_vec())),
		),
		(
			"key2".to_owned(),
			Some(ByteString::from(hex!("22222222222222222222222222222222").to_vec())),
		),
	];
	assert_eq!(result, expected);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_delete(pool: PgPool) {
	let source = make_source(pool);
	let result = source.get_mapping_changes(Some(BLOCK_1), BLOCK_4, scripts()).await;
	let expected = vec![("key1".to_owned(), None)];
	assert_eq!(result.unwrap(), expected);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_cached_governed_map_delete(pool: PgPool) {
	let source = make_cached_source(pool);
	let result = source.get_mapping_changes(Some(BLOCK_1), BLOCK_4, scripts()).await;
	let expected = vec![("key1".to_owned(), None)];
	assert_eq!(result.unwrap(), expected);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_upsert(pool: PgPool) {
	let source = make_source(pool);
	let mut result = source.get_mapping_changes(Some(BLOCK_6), BLOCK_7, scripts()).await.unwrap();
	result.sort();
	let expected = vec![(
		"key3".to_owned(),
		Some(ByteString::from(hex!("44444444444444444444444444444444").to_vec())),
	)];
	assert_eq!(result, expected);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_cached_governed_map_upsert(pool: PgPool) {
	let source = make_cached_source(pool);
	let mut result = source.get_mapping_changes(Some(BLOCK_6), BLOCK_7, scripts()).await.unwrap();
	result.sort();
}

fn scripts() -> MainChainScriptsV1 {
	MainChainScriptsV1 {
		asset_policy_id: PolicyId(hex!("500000000000000000000000000000000000434845434b504f494e69")),
		validator_address: MainchainAddress::from_str("governed_map_test_address").unwrap(),
	}
}

fn make_source(pool: PgPool) -> GovernedMapDataSourceImpl {
	GovernedMapDataSourceImpl { pool, metrics_opt: Some(test_metrics()) }
}

fn make_cached_source(pool: PgPool) -> GovernedMapDataSourceCachedImpl {
	GovernedMapDataSourceCachedImpl {
		pool,
		metrics_opt: Some(test_metrics()),
		cache_size: 10u16,
		security_parameter: 1u32,
		cache: Arc::new(Mutex::new(Cache::default())),
	}
}
