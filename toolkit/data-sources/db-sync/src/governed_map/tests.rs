use super::GovernedMapDataSourceImpl;
use crate::metrics::mock::test_metrics;
use hex_literal::hex;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sqlx::PgPool;
use std::str::FromStr;
use tokio_test::assert_err;

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_fails_on_wrong_block_hash(pool: PgPool) {
	let source = make_source(pool);
	let mc_block =
		McBlockHash(hex!("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
	let result = source.get_current_mappings(mc_block, scripts()).await;
	assert_err!(result);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_insert(pool: PgPool) {
	let source = make_source(pool);
	let mc_block =
		McBlockHash(hex!("B702000000000000000000000000000000000000000000000000000000000002"));
	let result = source.get_current_mappings(mc_block, scripts()).await;
	let mut expected: BTreeMap<String, ByteString> = BTreeMap::new();
	expected.insert(
		"key1".to_owned(),
		ByteString::from(hex!("11111111111111111111111111111111").to_vec()),
	);
	expected.insert(
		"key2".to_owned(),
		ByteString::from(hex!("22222222222222222222222222222222").to_vec()),
	);
	assert_eq!(result.unwrap(), expected);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_delete(pool: PgPool) {
	let source = make_source(pool);
	let mc_block =
		McBlockHash(hex!("B702000000000000000000000000000000000000000000000000000000000005"));
	let result = source.get_current_mappings(mc_block, scripts()).await;
	let mut expected: BTreeMap<String, ByteString> = BTreeMap::new();
	expected.insert(
		"key2".to_owned(),
		ByteString::from(hex!("22222222222222222222222222222222").to_vec()),
	);
	assert_eq!(result.unwrap(), expected);
}

#[sqlx::test(migrations = "./testdata/governed-map/migrations")]
async fn test_governed_map_upsert(pool: PgPool) {
	let source = make_source(pool);
	let mc_block =
		McBlockHash(hex!("B702000000000000000000000000000000000000000000000000000000000008"));
	let result = source.get_current_mappings(mc_block, scripts()).await;
	let mut expected: BTreeMap<String, ByteString> = BTreeMap::new();
	expected.insert(
		"key2".to_owned(),
		ByteString::from(hex!("22222222222222222222222222222222").to_vec()),
	);
	expected.insert(
		"key3".to_owned(),
		ByteString::from(hex!("44444444444444444444444444444444").to_vec()),
	);
	assert_eq!(result.unwrap(), expected);
}

fn scripts() -> MainChainScriptsV1 {
	MainChainScriptsV1 {
		asset: AssetId {
			policy_id: PolicyId::from_hex_unsafe(
				"500000000000000000000000000000000000434845434b504f494e69",
			),
			asset_name: AssetName::empty(),
		},
		validator_address: MainchainAddress::from_str("governed_map_test_address").unwrap(),
	}
}

fn make_source(pool: PgPool) -> GovernedMapDataSourceImpl {
	GovernedMapDataSourceImpl { pool, metrics_opt: Some(test_metrics()) }
}
