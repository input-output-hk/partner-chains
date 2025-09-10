use super::TokenBridgeDataSourceImpl;
use hex_literal::hex;
use sidechain_domain::{
	AssetName, MainchainAddress, McBlockHash, McBlockNumber, McTxHash, PolicyId, UtxoId,
	byte_string::ByteString,
};
use sp_partner_chains_bridge::{
	BridgeDataCheckpoint, BridgeTransferV1, MainChainScripts, TokenBridgeDataSource,
};
use sqlx::PgPool;
use std::str::FromStr;

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

fn block_4_hash() -> McBlockHash {
	McBlockHash(hex!("b000000000000000000000000000000000000000000000000000000000000004"))
}

fn block_8_hash() -> McBlockHash {
	McBlockHash(hex!("b000000000000000000000000000000000000000000000000000000000000008"))
}

fn init_ics_tx_hash() -> McTxHash {
	McTxHash(hex!("c000000000000000000000000000000000000000000000000000000000000001"))
}

fn last_ics_init_utxo() -> UtxoId {
	UtxoId::new(init_ics_tx_hash().0, 3)
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

fn invalid_transfer_1() -> BridgeTransferV1<ByteString> {
	BridgeTransferV1::InvalidTransfer {
		// invalid transfer consumes utxo from user transfer 2
		token_amount: 1000 - 120,
		utxo_id: invalid_transfer_1_utxo(),
	}
}

fn user_transfer_1_utxo() -> UtxoId {
	UtxoId::new(hex!("c000000000000000000000000000000000000000000000000000000000000003"), 0)
}

fn invalid_transfer_1_utxo() -> UtxoId {
	UtxoId::new(hex!("c000000000000000000000000000000000000000000000000000000000000005"), 0)
}

fn main_chain_scripts() -> MainChainScripts {
	MainChainScripts {
		token_policy_id: token_policy_id(),
		token_asset_name: token_asset_name(),
		illiquid_circulation_supply_validator_address:
			illiquid_circulation_supply_validator_address(),
	}
}

macro_rules! with_migration_versions {
	($(async fn $name:ident($pool:ident: PgPool) $body:block )*) => {
		$(
		mod $name {
				use super::*;
				use pretty_assertions::assert_eq;

				async fn $name($pool: PgPool) $body

				#[sqlx::test(migrations = "./testdata/bridge/migrations-tx-in-enabled")]
				async fn tx_in_enabled($pool: PgPool) {
					$name($pool).await
				}

				#[sqlx::test(migrations = "./testdata/bridge/migrations-tx-in-consumed")]
				async fn tx_in_consumed($pool: PgPool) {
					$name($pool).await
				}
		}
		)*
	}
}

with_migration_versions! {
	async fn gets_transfers_from_init_to_block_2(pool: PgPool) {
		let data_source: &dyn TokenBridgeDataSource<ByteString> =
			&TokenBridgeDataSourceImpl::new(pool, None);
		let data_checkpoint = BridgeDataCheckpoint::Utxo(last_ics_init_utxo());
		let current_mc_block = block_2_hash();
		let max_transfers = 2;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's two transfers done in block 2
		assert_eq!(transfers, vec![reserve_transfer(), user_transfer_1()]);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Utxo(user_transfer_1_utxo()))
	}

	async fn gets_transfers_from_init_to_block_4(pool: PgPool) {
		let data_source: &dyn TokenBridgeDataSource<ByteString> =
			&TokenBridgeDataSourceImpl::new(pool, None);
		let data_checkpoint = BridgeDataCheckpoint::Utxo(last_ics_init_utxo());
		let current_mc_block = block_4_hash();
		let max_transfers = 4;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's three valid transfers and one invalid done between blocks 2 and 4
		assert_eq!(
			transfers,
			vec![reserve_transfer(), user_transfer_1(), user_transfer_2(), invalid_transfer_1()]
		);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Utxo(invalid_transfer_1_utxo()))
	}

	async fn accepts_block_checkpoint(pool: PgPool) {
		let data_source: &dyn TokenBridgeDataSource<ByteString> =
			&TokenBridgeDataSourceImpl::new(pool, None);
		let data_checkpoint = BridgeDataCheckpoint::Block(McBlockNumber(1));
		let current_mc_block = block_4_hash();
		let max_transfers = 4;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		// There's three valid transfers and one invalid done between blocks 2 and 4
		assert_eq!(
			transfers,
			vec![reserve_transfer(), user_transfer_1(), user_transfer_2(), invalid_transfer_1()]
		);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Utxo(invalid_transfer_1_utxo()))
	}

	async fn returns_block_checkpoint_when_no_transfers_are_found(pool: PgPool) {
		let data_source: &dyn TokenBridgeDataSource<ByteString> =
			&TokenBridgeDataSourceImpl::new(pool, None);
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

	async fn returns_block_checkpoint_when_less_than_maximum_transfers_found(pool: PgPool) {
		let data_source: &dyn TokenBridgeDataSource<ByteString> =
			&TokenBridgeDataSourceImpl::new(pool, None);
		let data_checkpoint = BridgeDataCheckpoint::Block(McBlockNumber(0));
		let current_mc_block = block_8_hash();
		let max_transfers = 32;

		let (transfers, new_checkpoint) = data_source
			.get_transfers(main_chain_scripts(), data_checkpoint, max_transfers, current_mc_block)
			.await
			.unwrap();

		assert_eq!(
			transfers,
			vec![reserve_transfer(), user_transfer_1(), user_transfer_2(), invalid_transfer_1()]
		);

		assert_eq!(new_checkpoint, BridgeDataCheckpoint::Block(McBlockNumber(8)))
	}
}
