//! Db-Sync data source used by the Partner Chain token bridge observability
//!
//! # Assumptions
//!
//! The data source implementation assumes that the utxos found at the illiquid circulating
//! supply address conform to rules that are enforced by the Partner Chains smart contracts.
//!
//! Most importantly, transactions that spend any UTXOs from the ICS can only create exactly
//! one new UTXO at the ICS address. Conversely, transactions that create more than one UTXO
//! at the illiquid supply address can only spend UTXOs from outside of it. This guarantees
//! that the observability layer can always correctly identify the number of tokens transfered
//! by calculating the delta of `tokens in the new UTXO` - `tokens in the old ICS UTXOs`.

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

			let current_mc_block = get_block_by_hash(&self.pool, current_mc_block_hash.clone())
				.await?
				.ok_or(format!("Could not find block for hash {current_mc_block_hash:?}"))?;

			let data_checkpoint = match data_checkpoint {
				BridgeDataCheckpoint::Utxo(utxo) => {
					let TxBlockInfo { block_number, tx_ix } =
						get_block_info_for_utxo(&self.pool, utxo.tx_hash.into()).await?.ok_or(
							format!(
								"Could not find block info for data checkpoint: {data_checkpoint:?}"
							),
						)?;
					BridgeCheckpoint::Utxo {
						block_number: block_number.0,
						tx_ix: tx_ix.0,
						tx_out_ix: utxo.index.0,
					}
				},
				BridgeDataCheckpoint::Block(checkpoint_block_number) => {
					BridgeCheckpoint::Block { number: checkpoint_block_number.0 }
				},
			};

			let utxos = get_bridge_utxos_tx(
				self.db_sync_config.get_tx_in_config().await?,
				&self.pool,
				&main_chain_scripts.illiquid_supply_validator_address.into(),
				asset,
				data_checkpoint,
				current_mc_block.block_no,
				max_transfers,
			)
			.await?;

			let new_checkpoint = match utxos.last() {
				None => BridgeDataCheckpoint::Block(current_mc_block.block_no.into()),
				Some(_) if (utxos.len() as u32) < max_transfers => {
					BridgeDataCheckpoint::Block(current_mc_block.block_no.into())
				},
				Some(utxo) => BridgeDataCheckpoint::Utxo(utxo.utxo_id()),
			};

			let transfers = utxos.into_iter().flat_map(utxo_to_transfer).collect();

			Ok((transfers, new_checkpoint))
		}
	}
);

fn utxo_to_transfer<RecipientAddress>(
	utxo: BridgeUtxo,
) -> Option<BridgeTransferV1<RecipientAddress>>
where
	RecipientAddress: for<'a> TryFrom<&'a [u8]>,
{
	let token_delta = (utxo.tokens_out.0 as i128) - (utxo.tokens_in.0 as i128);

	if token_delta <= 0 {
		return None;
	}

	let token_amount = token_delta as u64;

	let transfer = match TokenTransferDatum::try_from(utxo.datum.0.clone()) {
		Ok(TokenTransferDatum::V1(TokenTransferDatumV1::UserTransfer { receiver })) => {
			match RecipientAddress::try_from(receiver.0.as_ref()) {
				Ok(recipient) => BridgeTransferV1::UserTransfer { token_amount, recipient },
				Err(_) => {
					BridgeTransferV1::InvalidTransfer { token_amount, utxo_id: utxo.utxo_id() }
				},
			}
		},
		Ok(TokenTransferDatum::V1(TokenTransferDatumV1::ReserveTransfer)) => {
			BridgeTransferV1::ReserveTransfer { token_amount }
		},
		Err(_) => BridgeTransferV1::InvalidTransfer { token_amount, utxo_id: utxo.utxo_id() },
	};

	Some(transfer)
}
