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

pub fn genesis_hash() -> McBlockHash {
	block_hash(0)
}

fn make_source(pool: PgPool) -> NativeTokenManagementDataSourceImpl {
	NativeTokenManagementDataSourceImpl { pool, metrics_opt: None }
}

#[sqlx::test(migrations = "./testdata/native-token/migrations")]
async fn defaults_to_zero_when_there_are_no_transfers(pool: PgPool) {
	let source = make_source(pool);
	let after_block = None;
	let to_block = genesis_hash();
	let result = source
		.get_token_transfer_events(
			after_block,
			to_block,
			native_token_policy_id(),
			native_token_asset_name(),
			illiquid_supply_address(),
		)
		.await
		.unwrap();

	assert_eq!(result.0, 0)
}

#[sqlx::test(migrations = "./testdata/native-token/migrations")]
async fn gets_sum_of_all_transfers_when_queried_up_to_latest_block(pool: PgPool) {
	let source = make_source(pool);
	let after_block = None;
	let to_block = block_hash(5);
	let result = source
		.get_token_transfer_events(
			after_block,
			to_block,
			native_token_policy_id(),
			native_token_asset_name(),
			illiquid_supply_address(),
		)
		.await
		.unwrap();

	assert_eq!(result.0, 11 + 12 + 13 + 14)
}

#[sqlx::test(migrations = "./testdata/native-token/migrations")]
async fn gets_sum_of_transfers_in_range(pool: PgPool) {
	let source = make_source(pool);
	let after_block = Some(block_hash(1));
	let to_block = block_hash(5);
	let result = source
		.get_token_transfer_events(
			after_block,
			to_block,
			native_token_policy_id(),
			native_token_asset_name(),
			illiquid_supply_address(),
		)
		.await
		.unwrap();

	assert_eq!(result.0, 12 + 13 + 14)
}
