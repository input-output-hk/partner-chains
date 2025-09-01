//! Db-Sync data source used by the Partner Chain token bridge observability

use crate::McFollowerMetrics;
use crate::db_model::*;
use crate::observed_async_trait;
use partner_chains_plutus_data::bridge::TokenTransferDatum;
use partner_chains_plutus_data::bridge::TokenTransferDatumV1;
use sidechain_domain::McBlockHash;
use sp_partner_chains_bridge::*;
use sqlx::PgPool;
use std::fmt::Debug;

#[cfg(test)]
mod tests;

/// Db-Sync data source serving data for Partner Chains token bridge
pub struct TokenBridgeDataSourceImpl {
	/// Postgres connection pool
	pool: PgPool,
	/// Prometheus metrics client
	metrics_opt: Option<McFollowerMetrics>,
	/// Configuration used by Db-Sync
	db_sync_config: DbSyncConfigurationProvider,
}

impl TokenBridgeDataSourceImpl {
	/// Crates a new token bridge data source
	pub fn new(pool: PgPool, metrics_opt: Option<McFollowerMetrics>) -> Self {
		Self { db_sync_config: DbSyncConfigurationProvider::new(pool.clone()), pool, metrics_opt }
	}
}

observed_async_trait!(
	impl<RecipientAddress> TokenBridgeDataSource<RecipientAddress> for TokenBridgeDataSourceImpl
	where
		RecipientAddress: Debug,
		RecipientAddress: (for<'a> TryFrom<&'a [u8]>),
	{
		async fn get_transfers(
			&self,
			main_chain_scripts: MainChainScripts,
			data_checkpoint: BridgeDataCheckpoint,
			max_transfers: u32,
			current_mc_block_hash: McBlockHash,
		) -> Result<
			(Vec<BridgeTransferV1<RecipientAddress>>, BridgeDataCheckpoint),
			Box<dyn std::error::Error + Send + Sync>,
		> {
			let asset = Asset {
				policy_id: main_chain_scripts.token_policy_id.into(),
				asset_name: main_chain_scripts.token_asset_name.into(),
			};
			let TxBlockInfo { block_number, tx_ix, tx_out_ix, .. } = get_block_info_for_utxo(
				&self.pool,
				data_checkpoint.0.tx_hash.into(),
				data_checkpoint.0.index.into(),
			)
			.await?
			.ok_or(format!("Could not find block info for data checkpoint: {data_checkpoint:?}"))?;
			let current_mc_block = get_block_by_hash(&self.pool, current_mc_block_hash.clone())
				.await?
				.ok_or(format!("Could not find block for hash {current_mc_block_hash:?}"))?;
			let utxos = get_bridge_utxos_tx(
				self.db_sync_config.get_tx_in_config().await?,
				&self.pool,
				&main_chain_scripts.illiquid_supply_validator_address.into(),
				asset,
				(block_number, tx_ix, tx_out_ix),
				current_mc_block.block_no,
				max_transfers,
			)
			.await?;

			let mut transfers = vec![];

			for utxo in &utxos {
				let token_delta = (utxo.tokens_out.0 - utxo.tokens_in.0) as u64;
				if token_delta > 0 {
					let transfer = match TokenTransferDatum::try_from(utxo.datum.0.clone()) {
						Ok(TokenTransferDatum::V1(TokenTransferDatumV1::UserTransfer {
							receiver,
						})) => match RecipientAddress::try_from(receiver.0.as_ref()) {
							Ok(recipient) => BridgeTransferV1::UserTransfer {
								token_amount: token_delta,
								recipient,
							},
							Err(_) => BridgeTransferV1::InvalidTransfer {
								token_amount: token_delta,
								utxo_id: utxo.utxo_id(),
							},
						},
						Ok(TokenTransferDatum::V1(TokenTransferDatumV1::ReserveTransfer)) => {
							BridgeTransferV1::ReserveTransfer { token_amount: token_delta }
						},
						Err(_) => BridgeTransferV1::InvalidTransfer {
							token_amount: token_delta,
							utxo_id: utxo.utxo_id(),
						},
					};

					transfers.push(transfer);
				}
			}

			let new_checkpoint = match utxos.last() {
				None => data_checkpoint,
				Some(utxo) => BridgeDataCheckpoint(utxo.utxo_id()),
			};

			Ok((transfers, new_checkpoint))
		}
	}
);
