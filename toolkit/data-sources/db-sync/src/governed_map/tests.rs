use super::{Cache, GovernedMapDataSourceCachedImpl, GovernedMapDataSourceImpl};
use crate::block::{BlockDataSourceImpl, DbSyncBlockDataSourceConfig};
use crate::db_model::{DbSyncConfigurationProvider, TxInConfiguration};
use crate::metrics::mock::test_metrics;
use hex_literal::hex;
use pretty_assertions::assert_eq;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::mainchain_epoch::{Duration, MainchainEpochConfig, Timestamp};
use sidechain_domain::*;
use sp_governed_map::{GovernedMapDataSource, MainChainScriptsV1};
use sqlx::PgPool;
use std::cell::OnceCell;
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

macro_rules! with_migration_versions {
	($(async fn $name:ident($pool:ident: PgPool, $tx_in_cfg:ident: TxInConfiguration) $body:block )*) => {

		$(
            paste::paste!(
				async fn $name($pool: PgPool, $tx_in_cfg: TxInConfiguration) $body

				#[sqlx::test(migrations = "./testdata/governed-map/migrations-tx-in-enabled")]
				async fn [<$name _v1>]($pool: PgPool) {
					$name($pool, TxInConfiguration::Enabled).await
				}

				#[sqlx::test(migrations = "./testdata/governed-map/migrations-tx-in-consumed")]
				async fn [<$name _v2>]($pool: PgPool) {
					$name($pool, TxInConfiguration::Consumed).await
				}
			);
		)*
    };
}

with_migration_versions! {
	async fn test_governed_map_fails_on_wrong_block_hash(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_source(pool, tx_in_config);
		let mc_block =
			McBlockHash(hex!("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
		let result = source.get_mapping_changes(None, mc_block, scripts()).await;
		assert_err!(result);
	}

	async fn test_cached_governed_map_fails_on_wrong_block_hash(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_cached_source(pool, tx_in_config).await;
		let mc_block =
			McBlockHash(hex!("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"));
		let result = source.get_mapping_changes(None, mc_block, scripts()).await;
		assert_err!(result);
	}

	async fn test_governed_map_insert(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_source(pool, tx_in_config);
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

	async fn test_cached_governed_map_insert(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_cached_source(pool, tx_in_config).await;
		let result = source.get_mapping_changes(None, BLOCK_1, scripts()).await.unwrap();

		let expected = vec![
			(
				"key2".to_owned(),
				Some(ByteString::from(hex!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").to_vec())),
			),
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

	async fn test_governed_map_delete(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_source(pool, tx_in_config);
		let result = source.get_mapping_changes(Some(BLOCK_1), BLOCK_4, scripts()).await;
		let expected = vec![("key1".to_owned(), None)];
		assert_eq!(result.unwrap(), expected);
	}

	async fn test_cached_governed_map_delete(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_cached_source(pool, tx_in_config).await;
		let result = source.get_mapping_changes(Some(BLOCK_1), BLOCK_4, scripts()).await;
		let expected = vec![("key1".to_owned(), None)];
		assert_eq!(result.unwrap(), expected);
	}

	async fn test_governed_map_upsert(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_source(pool, tx_in_config);
		let mut result = source.get_mapping_changes(Some(BLOCK_6), BLOCK_7, scripts()).await.unwrap();
		result.sort();
		let expected = vec![(
			"key3".to_owned(),
			Some(ByteString::from(hex!("44444444444444444444444444444444").to_vec())),
		)];
		assert_eq!(result, expected);
	}

	async fn test_cached_governed_map_upsert(pool: PgPool, tx_in_config: TxInConfiguration) {
		let source = make_cached_source(pool, tx_in_config).await;
		let mut result = source.get_mapping_changes(Some(BLOCK_6), BLOCK_7, scripts()).await.unwrap();
		result.sort();
		let expected = vec![
			("key3".to_owned(), None),
			(
				"key3".to_owned(),
				Some(ByteString::from(hex!("44444444444444444444444444444444").to_vec())),
			),
		];
		assert_eq!(result, expected);
	}
}

fn scripts() -> MainChainScriptsV1 {
	MainChainScriptsV1 {
		asset_policy_id: PolicyId(hex!("500000000000000000000000000000000000434845434b504f494e69")),
		validator_address: MainchainAddress::from_str("governed_map_test_address").unwrap(),
	}
}

fn make_source(pool: PgPool, tx_in_config: TxInConfiguration) -> GovernedMapDataSourceImpl {
	GovernedMapDataSourceImpl {
		pool: pool.clone(),
		metrics_opt: Some(test_metrics()),
		db_sync_config: DbSyncConfigurationProvider {
			pool,
			tx_in_config: Arc::new(tokio::sync::Mutex::new(OnceCell::from(tx_in_config))),
		},
	}
}

async fn make_cached_source(
	pool: PgPool,
	tx_in_config: TxInConfiguration,
) -> GovernedMapDataSourceCachedImpl {
	GovernedMapDataSourceCachedImpl {
		pool: pool.clone(),
		metrics_opt: Some(test_metrics()),
		cache_size: 10u16,
		cache: Arc::new(Mutex::new(Cache::default())),
		blocks: Arc::new(BlockDataSourceImpl::from_config(
			pool.clone(),
			DbSyncBlockDataSourceConfig {
				cardano_security_parameter: 432,
				cardano_active_slots_coeff: 0.05,
				block_stability_margin: 0,
			},
			&mainchain_epoch_config(),
		)),
		db_sync_config: DbSyncConfigurationProvider {
			pool,
			tx_in_config: Arc::new(tokio::sync::Mutex::new(OnceCell::from(tx_in_config))),
		},
	}
}

fn mainchain_epoch_config() -> MainchainEpochConfig {
	MainchainEpochConfig {
		first_epoch_timestamp_millis: Timestamp::from_unix_millis(1650558070000),
		epoch_duration_millis: Duration::from_millis(1000 * 1000),
		first_epoch_number: 189,
		first_slot_number: 189000,
		slot_duration_millis: Duration::from_millis(1000),
	}
}
