use crate::bridge::cache::CachedTokenBridgeDataSourceImpl;
use crate::{BlockDataSourceImpl, DbSyncBlockDataSourceConfig, TokenBridgeDataSourceImpl};
use hex_literal::hex;
use sidechain_domain::byte_string::ByteString;
use sidechain_domain::mainchain_epoch::{Duration, MainchainEpochConfig, Timestamp};
use sidechain_domain::{
	AssetName, MainchainAddress, McBlockHash, McBlockNumber, McTxHash, PolicyId,
};
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
use sqlx::PgPool;
use std::str::FromStr;
use std::sync::Arc;

fn token_policy_id() -> PolicyId {
	PolicyId(hex!("500000000000000000000000000000000000434845434b504f494e69"))
}

fn token_asset_name() -> AssetName {
	AssetName(b"native token".to_vec().try_into().unwrap())
}

fn illiquid_circulation_supply_validator_address() -> MainchainAddress {
	MainchainAddress::from_str("ics address").unwrap()
}

fn block_2_hash() -> McBlockHash {
	McBlockHash(hex!("b000000000000000000000000000000000000000000000000000000000000002"))
}

fn block_3_hash() -> McBlockHash {
	McBlockHash(hex!("b000000000000000000000000000000000000000000000000000000000000003"))
}

fn block_4_hash() -> McBlockHash {
	McBlockHash(hex!("b000000000000000000000000000000000000000000000000000000000000004"))
}

fn block_8_hash() -> McBlockHash {
	McBlockHash(hex!("b000000000000000000000000000000000000000000000000000000000000008"))
}

fn init_ics_tx_hash() -> McTxHash {
	McTxHash(hex!("c000000000000000000000000000000000000000000000000000000000000001"))
}

fn reserve_transfer() -> BridgeTransferV1<ByteString> {
	BridgeTransferV1::<ByteString>::ReserveTransfer { token_amount: 100 }
}

fn user_transfer_1() -> BridgeTransferV1<ByteString> {
	BridgeTransferV1::UserTransfer {
		// user transfer 1 consumes utxo from reserve transfer
		token_amount: 110 - 100,
		recipient: ByteString(hex!("abcd").to_vec()),
	}
}

fn user_transfer_2() -> BridgeTransferV1<ByteString> {
	BridgeTransferV1::UserTransfer {
		// user transfer 2 consumes utxo from user transfer 1
		token_amount: 120 - 110,
		recipient: ByteString(hex!("1234").to_vec()),
	}
}

// transfer with invalid datum
fn invalid_transfer_1() -> BridgeTransferV1<ByteString> {
	BridgeTransferV1::InvalidTransfer {
		// invalid transfer consumes utxo from user transfer 2
		token_amount: 1000 - 120,
		tx_hash: invalid_transfer_1_tx(),
	}
}

// transfer with no datum
fn invalid_transfer_2() -> BridgeTransferV1<ByteString> {
	BridgeTransferV1::InvalidTransfer { token_amount: 1000, tx_hash: invalid_transfer_2_tx() }
}

fn reserve_transfer_tx() -> McTxHash {
	McTxHash(hex!("c000000000000000000000000000000000000000000000000000000000000002"))
}

fn user_transfer_1_tx() -> McTxHash {
	McTxHash(hex!("c000000000000000000000000000000000000000000000000000000000000003"))
}

fn invalid_transfer_1_tx() -> McTxHash {
	McTxHash(hex!("c000000000000000000000000000000000000000000000000000000000000005"))
}

fn invalid_transfer_2_tx() -> McTxHash {
	McTxHash(hex!("c000000000000000000000000000000000000000000000000000000000000006"))
}

fn main_chain_scripts() -> MainChainScripts {
	MainChainScripts {
		token_policy_id: token_policy_id(),
		token_asset_name: token_asset_name(),
		illiquid_circulation_supply_validator_address:
			illiquid_circulation_supply_validator_address(),
	}
}

macro_rules! with_migration_versions_and_caching {
	($(async fn $name:ident($data_source:ident: &dyn TokenBridgeDataSource<ByteString>) $body:block )*) => {
		$(
		mod $name {
			use super::*;
			#[allow(unused_imports)]
			use pretty_assertions::assert_eq;

			async fn $name($data_source: &dyn TokenBridgeDataSource<ByteString>) $body

			mod uncached {
				use super::*;
				#[allow(unused_imports)]
				use pretty_assertions::assert_eq;

				#[sqlx::test(migrations = "./testdata/bridge/migrations-tx-in-enabled")]
				async fn tx_in_enabled(pool: PgPool) {
					$name(&create_data_source(pool)).await
				}

				#[sqlx::test(migrations = "./testdata/bridge/migrations-tx-in-consumed")]
				async fn tx_in_consumed(pool: PgPool) {
					$name(&create_data_source(pool)).await
				}
			}

			mod cached {
				use super::*;

				#[sqlx::test(migrations = "./testdata/bridge/migrations-tx-in-enabled")]
				async fn tx_in_enabled(pool: PgPool) {
					$name(&create_cached_source(pool)).await
				}

				#[sqlx::test(migrations = "./testdata/bridge/migrations-tx-in-consumed")]
				async fn tx_in_consumed(pool: PgPool) {
					$name(&create_cached_source(pool)).await
				}
			}

		}
		)*
	}
}

fn main_chain_epoch_config() -> MainchainEpochConfig {
	MainchainEpochConfig {
		first_epoch_timestamp_millis: Timestamp::from_unix_millis(1650558070000),
		epoch_duration_millis: Duration::from_millis(1000 * 1000),
		first_epoch_number: 189,
		first_slot_number: 189000,
		slot_duration_millis: Duration::from_millis(1000),
	}
}

fn block_data_source_config() -> DbSyncBlockDataSourceConfig {
	DbSyncBlockDataSourceConfig {
		cardano_security_parameter: 432,
		cardano_active_slots_coeff: 0.05,
		block_stability_margin: 0,
	}
}

fn create_data_source(pool: PgPool) -> TokenBridgeDataSourceImpl {
	TokenBridgeDataSourceImpl::new(pool, None)
}

fn create_cached_source(pool: PgPool) -> CachedTokenBridgeDataSourceImpl {
	let blocks = Arc::new(BlockDataSourceImpl::from_config(
		pool.clone(),
		block_data_source_config(),
		&main_chain_epoch_config(),
	));
	let cache_lookahead = 32;
	CachedTokenBridgeDataSourceImpl::new(pool, None, blocks, cache_lookahead)
}

with_migration_versions_and_caching! {
	async fn gets_transfers_from_init_to_block_2(data_source: &dyn TokenBridgeDataSource<ByteString>) {
		let data_checkpoint = BridgeDataCheckpoint::Tx(init_ics_tx_hash());
		let current_mc_block = block_2_hash();
		let max_transfers = 2;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's two transfers done in block 2
		assert_eq!(transfers, vec![reserve_transfer(), user_transfer_1()]);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Tx(user_transfer_1_tx()))
	}

	async fn gets_transfers_from_init_to_block_4(data_source: &dyn TokenBridgeDataSource<ByteString>) {
		let data_checkpoint = BridgeDataCheckpoint::Tx(init_ics_tx_hash());
		let current_mc_block = block_4_hash();
		let max_transfers = 5;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's three valid transfers and one invalid done between blocks 2 and 4
		assert_eq!(
			transfers,
			vec![reserve_transfer(), user_transfer_1(), user_transfer_2(), invalid_transfer_1(), invalid_transfer_2()]
		);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Tx(invalid_transfer_2_tx()))
	}

	async fn accepts_block_checkpoint(data_source: &dyn TokenBridgeDataSource<ByteString>) {
		let data_checkpoint = BridgeDataCheckpoint::Block(McBlockNumber(1));
		let current_mc_block = block_4_hash();
		let max_transfers = 5;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's three valid transfers and one invalid done between blocks 2 and 4
		assert_eq!(
			transfers,
			vec![reserve_transfer(), user_transfer_1(), user_transfer_2(), invalid_transfer_1(), invalid_transfer_2()]
		);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Tx(invalid_transfer_2_tx()))
	}

	async fn returns_block_checkpoint_when_no_transfers_are_found(data_source: &dyn TokenBridgeDataSource<ByteString>) {
		let data_checkpoint = BridgeDataCheckpoint::Block(McBlockNumber(6));
		let current_mc_block = block_8_hash();
		let max_transfers = 32;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		assert_eq!(transfers, vec![]);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Block(McBlockNumber(8)))
	}

	async fn returns_block_checkpoint_when_less_than_maximum_transfers_found(data_source: &dyn TokenBridgeDataSource<ByteString>) {
		let data_checkpoint = BridgeDataCheckpoint::Block(McBlockNumber(0));
		let current_mc_block = block_8_hash();
		let max_transfers = 32;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		assert_eq!(
			transfers,
			vec![reserve_transfer(), user_transfer_1(), user_transfer_2(), invalid_transfer_1(), invalid_transfer_2()]
		);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Block(McBlockNumber(8)))
	}

	async fn truncates_output_and_returns_utxo_checkpoint_if_max_output_is_reached(data_source: &dyn TokenBridgeDataSource<ByteString>) {
		let data_checkpoint = BridgeDataCheckpoint::Tx(init_ics_tx_hash());
		let current_mc_block = block_2_hash();
		let max_transfers = 1;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's two transfers done in block 2
		assert_eq!(transfers, vec![reserve_transfer()]);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Tx(reserve_transfer_tx()))
	}

	async fn utxos_from_checkpoint_block_are_not_included_in_result(data_source: &dyn TokenBridgeDataSource<ByteString>) {
		let data_checkpoint = BridgeDataCheckpoint::Block(McBlockNumber(2));
		let current_mc_block = block_3_hash();
		let max_transfers = 10;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's two transfers done in block 2
		assert_eq!(transfers, vec![]);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Block(McBlockNumber(3)))
	}
}
