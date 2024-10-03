use super::NativeTokenManagementDataSourceImpl;
use main_chain_follower_api::NativeTokenManagementDataSource;
use sidechain_domain::{AssetName, MainchainAddress, McBlockHash, PolicyId};
use sqlx::PgPool;
use std::str::FromStr;

fn native_token_policy_id() -> PolicyId {
	PolicyId::from_hex_unsafe("6c969320597b755454ff3653ad09725d590c570827a129aeb4385526")
}

fn native_token_asset_name() -> AssetName {
	AssetName::from_hex_unsafe("546573744275647a507265766965775f3335")
}

fn illiquid_supply_address() -> MainchainAddress {
	MainchainAddress::from_str("addr_test1wrhvtvx3f0g9wv9rx8kfqc60jva3e07nqujk2cspekv4mqs9rjdvz")
		.unwrap()
}

fn block_hash(i: u32) -> McBlockHash {
	McBlockHash::from_str(&format!(
		"b00000000000000000000000000000000000000000000000000000000000000{i}"
	))
	.unwrap()
}

fn make_source(pool: PgPool) -> NativeTokenManagementDataSourceImpl {
	NativeTokenManagementDataSourceImpl::test_new(pool, None, 1u32, 10u16)
}

#[sqlx::test(migrations = "./testdata/native-token/migrations")]
async fn defaults_to_zero_when_there_are_no_transfers(pool: PgPool) {
	let source = make_source(pool);
	let genesis_block = 0;
	let result = run(&source, None, genesis_block).await;

	assert_eq!(result, 0)
}

#[sqlx::test(migrations = "./testdata/native-token/migrations")]
async fn gets_sum_of_all_transfers_when_queried_up_to_latest_block(pool: PgPool) {
	let source = make_source(pool);
	let result = run(&source, None, 5).await;

	assert_eq!(result, 11 + 12 + 13 + 14)
}

#[sqlx::test(migrations = "./testdata/native-token/migrations")]
async fn gets_sum_of_transfers_in_range(pool: PgPool) {
	let source = make_source(pool);
	let result = run(&source, Some(1), 5).await;

	assert_eq!(result, 12 + 13 + 14)
}

#[sqlx::test(migrations = "./testdata/native-token/migrations")]
async fn query_for_each_blocks_pair(pool: PgPool) {
	let source = make_source(pool.clone());
	// after is None, don't use nor fill cache
	let r1 = run(&source, None, 1).await;
	// this call will fill the cache
	let r2 = run(&source, Some(1), 2).await;
	// make sure DB is not used anymore
	delete_all_blocks_from_db(&pool).await;
	let r3 = run(&source, Some(2), 3).await;
	let r4 = run(&source, Some(3), 4).await;
	let r5 = run(&source, Some(4), 5).await;

	assert_eq!(vec![r1, r2, r3, r4, r5], vec![11, 0, 12, 0, 13 + 14])
}

async fn run(source: &NativeTokenManagementDataSourceImpl, from: Option<u32>, to: u32) -> u128 {
	source
		.get_total_native_token_transfer(
			from.map(block_hash),
			block_hash(to),
			native_token_policy_id(),
			native_token_asset_name(),
			illiquid_supply_address(),
		)
		.await
		.unwrap()
		.0
}

async fn delete_all_blocks_from_db(pool: &PgPool) {
	sqlx::query("DELETE FROM block").execute(pool).await.unwrap();
}
