use super::*;
use crate::BlockDataSourceImpl;
use crate::db_model::BridgeUtxo;
use futures::lock::Mutex;
use sidechain_domain::{MainchainBlock, McBlockHash, UtxoId};
use std::{cmp::min, collections::HashMap, error::Error, sync::Arc};

/// Bridge transfer data source with block range-based caching
///
/// This data source caches utxos in some range [from_block, to_block] (inclusive) and serves
/// queries from the cache. In case of a cache miss, the cache is first replaced before serving
/// data. The cache is filled with utxos in range:
///     [lower_query_bound, min(upper_query_bound + cache_lookahead, current_stable_tip)]
///
/// In case of queries where the lower bound is a UTXO, all UTXOs from the containing
/// block are stored. Technically servable case where the lower bound UTXO is the last one in its
/// block but the block is not stored, is treated as a cache miss.
pub struct CachedTokenBridgeDataSourceImpl {
	/// Postgres connection pool
	pool: PgPool,
	/// Prometheus metrics client
	metrics_opt: Option<McFollowerMetrics>,
	/// Configuration used by Db-Sync
	db_sync_config: DbSyncConfigurationProvider,
	/// [BlockDataSourceImpl] instance shared with other data sources for cache reuse.
	blocks: Arc<BlockDataSourceImpl>,
	/// Internal data cache
	cache: Arc<Mutex<TokenUtxoCache>>,
	/// Number of additional blocks that should be loaded into cache when refreshing
	cache_lookahead: u32,
}

#[derive(Default)]
pub(crate) struct TokenUtxoCache {
	mc_scripts: MainChainScripts,
	start_block: BlockNumber,
	end_block: BlockNumber,
	transfers: Vec<BridgeUtxo>,
	utxo_cache: HashMap<UtxoId, BridgeUtxo>,
}

impl TokenUtxoCache {
	pub(crate) fn new() -> Self {
		Self::default()
	}

	pub(crate) fn set_mc_scripts(&mut self, mc_scripts: MainChainScripts) {
		if self.mc_scripts != mc_scripts {
			self.mc_scripts = mc_scripts;
			self.transfers = vec![];
			self.start_block = BlockNumber(0);
			self.end_block = BlockNumber(0);
		}
	}

	pub(crate) fn set_cached_transfers(
		&mut self,
		start_block: BlockNumber,
		end_block: BlockNumber,
		transfers: Vec<BridgeUtxo>,
	) {
		self.start_block = start_block;
		self.end_block = end_block;
		self.utxo_cache = transfers.iter().map(|utxo| (utxo.utxo_id(), utxo.clone())).collect();
		self.transfers = transfers;
	}

	pub(crate) fn serve_from_cache(
		&self,
		checkpoint: &ResolvedBridgeDataCheckpoint,
		to_block: BlockNumber,
		max_transfers: u32,
	) -> Option<Vec<BridgeUtxo>> {
		if self.end_block < to_block {
			return None;
		}

		let skip_pred: Box<dyn FnMut(&&BridgeUtxo) -> bool> = match checkpoint {
			ResolvedBridgeDataCheckpoint::Block { number }
				if self.start_block <= number.saturating_add(1u32) =>
			{
				Box::new(move |utxo| *number >= utxo.block_number)
			},
			ResolvedBridgeDataCheckpoint::Utxo { block_number, tx_ix, tx_out_ix }
				if self.start_block <= *block_number =>
			{
				Box::new(move |utxo| utxo.ordering_key() <= (*block_number, *tx_ix, *tx_out_ix))
			},
			_ => return None,
		};

		Some(
			self.transfers
				.iter()
				.skip_while(skip_pred)
				.take_while(|utxo| to_block.0 >= utxo.block_number.0)
				.take(max_transfers as usize)
				.cloned()
				.collect(),
		)
	}

	pub(crate) fn try_resolve_checkpoint_from_cache(
		&self,
		utxo_id: &UtxoId,
	) -> Option<ResolvedBridgeDataCheckpoint> {
		let BridgeUtxo { block_number, tx_ix, utxo_ix, .. } =
			self.utxo_cache.get(utxo_id).cloned()?;

		Some(ResolvedBridgeDataCheckpoint::Utxo { block_number, tx_ix, tx_out_ix: utxo_ix })
	}
}

observed_async_trait!(
	impl<RecipientAddress> TokenBridgeDataSource<RecipientAddress> for CachedTokenBridgeDataSourceImpl
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
			self.set_cache_mc_scripts(main_chain_scripts.clone()).await;

			let to_block = self.get_block_by_hash(&current_mc_block_hash).await?.number.into();

			let data_checkpoint = self.resolve_data_checkpoint(&data_checkpoint).await?;

			let utxos =
				match self.try_serve_from_cache(&data_checkpoint, to_block, max_transfers).await {
					Some(utxos) => utxos,
					None => {
						self.fill_cache(main_chain_scripts, &data_checkpoint, to_block).await?;
						self.try_serve_from_cache(&data_checkpoint, to_block, max_transfers)
							.await
							.ok_or("Data should be present in cache after filling cache succeeded")?
					},
				};

			let new_checkpoint = match utxos.last() {
				Some(utxo) if (utxos.len() as u32) >= max_transfers => {
					BridgeDataCheckpoint::Utxo(utxo.utxo_id())
				},
				_ => BridgeDataCheckpoint::Block(to_block.into()),
			};

			let transfers = utxos.into_iter().flat_map(utxo_to_transfer).collect();

			Ok((transfers, new_checkpoint))
		}
	}
);

impl CachedTokenBridgeDataSourceImpl {
	/// Crates a new token bridge data source
	pub fn new(
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
		blocks: Arc<BlockDataSourceImpl>,
		cache_lookahead: u32,
	) -> Self {
		Self {
			db_sync_config: DbSyncConfigurationProvider::new(pool.clone()),
			pool,
			metrics_opt,
			blocks,
			cache: Arc::new(Mutex::new(TokenUtxoCache::new())),
			cache_lookahead,
		}
	}

	async fn set_cache_mc_scripts(&self, main_chain_scripts: MainChainScripts) {
		let mut cache = self.cache.lock().await;
		cache.set_mc_scripts(main_chain_scripts.clone());
	}

	async fn try_serve_from_cache(
		&self,
		data_checkpoint: &ResolvedBridgeDataCheckpoint,
		to_block: BlockNumber,
		max_transfers: u32,
	) -> Option<Vec<BridgeUtxo>> {
		let cache = self.cache.lock().await;
		cache.serve_from_cache(data_checkpoint, to_block, max_transfers)
	}

	async fn fill_cache(
		&self,
		main_chain_scripts: MainChainScripts,
		data_checkpoint: &ResolvedBridgeDataCheckpoint,
		to_block: BlockNumber,
	) -> Result<(), Box<dyn Error + Send + Sync>> {
		let from_block = data_checkpoint.get_block_number();

		// We want to load all data in the block of `data_checkpoint`, so we go one block back.
		let effective_data_checkpoint =
			ResolvedBridgeDataCheckpoint::Block { number: from_block.saturating_sub(1u32) };

		let latest_block = self.get_latest_stable_block().await?.unwrap_or(to_block);

		let to_block: BlockNumber =
			min(to_block.saturating_add(self.cache_lookahead), latest_block);

		let utxos = get_bridge_utxos_tx(
			self.db_sync_config.get_tx_in_config().await?,
			&self.pool,
			&main_chain_scripts.illiquid_circulation_supply_validator_address.clone().into(),
			main_chain_scripts.asset_id().into(),
			effective_data_checkpoint,
			to_block.into(),
			None,
		)
		.await?;

		self.set_cached_transfers(from_block, to_block, utxos).await;

		Ok(())
	}

	async fn set_cached_transfers(
		&self,
		start_block: BlockNumber,
		end_block: BlockNumber,
		utxos: Vec<BridgeUtxo>,
	) {
		let mut cache = self.cache.lock().await;
		cache.set_cached_transfers(start_block, end_block, utxos);
	}

	async fn get_latest_stable_block(
		&self,
	) -> Result<Option<BlockNumber>, Box<dyn Error + Send + Sync>> {
		let latest_block_timestamp = self.blocks.get_latest_block_info().await?.timestamp;
		Ok(self
			.blocks
			.get_latest_stable_block_for(latest_block_timestamp.into())
			.await?
			.map(|block| block.number.into()))
	}

	async fn resolve_checkpoint_for_utxo(
		&self,
		utxo_id: &UtxoId,
	) -> Result<ResolvedBridgeDataCheckpoint, Box<dyn Error + Send + Sync>> {
		let TxBlockInfo { block_number, tx_ix } =
			get_block_info_for_utxo(&self.pool, utxo_id.tx_hash.into())
				.await?
				.ok_or(format!("Could not find block info for utxo: {utxo_id:?}"))?;
		Ok(ResolvedBridgeDataCheckpoint::Utxo {
			block_number,
			tx_ix,
			tx_out_ix: utxo_id.index.into(),
		})
	}

	async fn resolve_data_checkpoint(
		&self,
		data_checkpoint: &BridgeDataCheckpoint,
	) -> Result<ResolvedBridgeDataCheckpoint, Box<dyn Error + Send + Sync>> {
		match data_checkpoint {
			BridgeDataCheckpoint::Block(number) => {
				Ok(ResolvedBridgeDataCheckpoint::Block { number: (*number).into() })
			},
			BridgeDataCheckpoint::Utxo(utxo) => {
				match self.cache.lock().await.try_resolve_checkpoint_from_cache(&utxo) {
					Some(checkpoint) => Ok(checkpoint),
					None => self.resolve_checkpoint_for_utxo(&utxo).await,
				}
			},
		}
	}

	async fn get_block_by_hash(
		&self,
		mc_block_hash: &McBlockHash,
	) -> Result<MainchainBlock, Box<dyn Error + Send + Sync>> {
		Ok(self
			.blocks
			.get_block_by_hash(mc_block_hash.clone())
			.await?
			.ok_or(format!("Could not find block for hash {mc_block_hash:?}"))?)
	}
}
