use crate::db_model::{Block, BlockNumber};
use crate::metrics::McFollowerMetrics;
use crate::observed_async_trait;
use crate::{DataSourceError, Result};
use derive_new::new;
use itertools::Itertools;
use sidechain_domain::*;
use sp_native_token_management::{MainChainScripts, NativeTokenManagementDataSource};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

#[derive(new)]
pub struct NativeTokenManagementDataSourceImpl {
	pub pool: PgPool,
	pub metrics_opt: Option<McFollowerMetrics>,
	security_parameter: u32,
	cache_size: u16,
	#[new(default)]
	cache: Arc<Mutex<Cache>>,
}

observed_async_trait!(
impl NativeTokenManagementDataSource for NativeTokenManagementDataSourceImpl {
	// after_block is always less or equal to_block
	// to_block is always a stable block
	async fn get_total_native_token_transfer(
		&self,
		after_block: Option<McBlockHash>,
		to_block: McBlockHash,
		scripts: MainChainScripts,
	) -> std::result::Result<NativeTokenAmount, Box<dyn std::error::Error + Send + Sync>> {
		if let Some(after_block) = after_block {
			if after_block == to_block {
				Ok(NativeTokenAmount(0))
			} else if let Some(amount) = self.get_from_cache(&after_block, &to_block, &scripts) {
				log::debug!(
					"Illiquid supply transfers sum from cache after block '{:?}' to block '{:?}' is {}",
					after_block, to_block, amount.0
				);
				Ok(amount)
			} else {
				log::debug!("Illiquid supply transfers after block '{:?}' to block '{:?}' not found in cache.", after_block, to_block);
				let block_to_amount = self
					.get_data_to_cache(&after_block, &to_block, &scripts)
					.await?;
				log::debug!("Caching illiquid supply transfers from {} blocks", block_to_amount.len());

				let amount = block_to_amount
					.iter()
					.skip(1) // the first element is the 'after_block' which is not included in the sum
					.take_while_inclusive(|(block_hash, _)| *block_hash != to_block)
					.map(|(_, amount)| amount)
					.sum();
				log::debug!("Amount of illiquid supply transfers is {}", amount);

				if let Ok(mut cache) = self.cache.lock() {
					cache.update(block_to_amount, scripts)
				}
				Ok(NativeTokenAmount(amount))
			}
		} else {
			let amount = self
				.query_transfers_from_genesis(&to_block, &scripts)
				.await?;
			log::debug!("Amount of illiquid supply transfers from genesis to {} is {}", to_block, amount.0);
			Ok(amount)
		}
	}
});

impl NativeTokenManagementDataSourceImpl {
	pub fn new_from_env(
		pool: PgPool,
		metrics_opt: Option<McFollowerMetrics>,
	) -> std::result::Result<Self, &'static str> {
		let security_parameter: u32 = std::env::var("CARDANO_SECURITY_PARAMETER")
			.ok()
			.and_then(|s| s.parse().ok())
			.ok_or("Couldn't read env variable CARDANO_SECURITY_PARAMETER as u32")?;
		Ok(Self {
			pool,
			metrics_opt,
			security_parameter,
			cache_size: 1000,
			cache: Default::default(),
		})
	}

	fn get_from_cache(
		&self,
		after_block: &McBlockHash,
		to_block: &McBlockHash,
		scripts: &MainChainScripts,
	) -> Option<NativeTokenAmount> {
		let cache = self.cache.lock().ok()?;
		if cache.scripts.as_ref() == Some(scripts) {
			cache.get_sum_in_range(after_block, to_block).map(NativeTokenAmount)
		} else {
			None
		}
	}

	// invariant: to_block is always a stable block
	// Returned data contains the 'from_block', it is required as guard for the cache
	async fn get_data_to_cache(
		&self,
		from_block: &McBlockHash,
		to_block: &McBlockHash,
		scripts: &MainChainScripts,
	) -> Result<Vec<(McBlockHash, u128)>> {
		let (from_block_no, to_block_no, latest_block) = futures::try_join!(
			get_from_block_no(from_block, &self.pool),
			get_to_block_no(to_block, &self.pool),
			get_latest_block(&self.pool),
		)?;
		let latest_stable_block = latest_block.block_no.0.saturating_sub(self.security_parameter);

		// to_block_no is always a stable block, so it is not above latest_stable_block,
		// but from_block_no + cache_size could be above latest_stable_block, so min has to be applied
		let cache_to_block_no = BlockNumber(std::cmp::min(
			latest_stable_block,
			std::cmp::max(to_block_no.0, from_block_no.0.saturating_add(self.cache_size.into())),
		));
		// transfers starts with block having hash equal to after_block or genesis
		let transfers = self.query_db(from_block_no, cache_to_block_no, scripts).await?;
		Ok(transfers.iter().map(|t| (McBlockHash(t.block_hash), t.amount.0)).collect())
	}

	async fn query_db(
		&self,
		from_block: BlockNumber,
		to_block: BlockNumber,
		scripts: &MainChainScripts,
	) -> Result<Vec<crate::db_model::BlockTokenAmount>> {
		let address = scripts.illiquid_supply_validator_address.clone().into();
		let asset = to_db_asset(scripts);
		Ok(crate::db_model::get_native_token_transfers(
			&self.pool, from_block, to_block, asset, address,
		)
		.await?)
	}

	async fn query_transfers_from_genesis(
		&self,
		to_block: &McBlockHash,
		scripts: &MainChainScripts,
	) -> Result<NativeTokenAmount> {
		let to_block = get_to_block_no(to_block, &self.pool).await?;

		Ok(crate::db_model::get_total_native_tokens_transfered(
			&self.pool,
			to_block,
			to_db_asset(scripts),
			scripts.illiquid_supply_validator_address.clone().into(),
		)
		.await?
		.into())
	}
}

fn to_db_asset(scripts: &MainChainScripts) -> crate::db_model::Asset {
	crate::db_model::Asset {
		policy_id: scripts.native_token_policy_id.clone().into(),
		asset_name: scripts.native_token_asset_name.clone().into(),
	}
}

async fn get_from_block_no(from_block: &McBlockHash, pool: &PgPool) -> Result<BlockNumber> {
	Ok(crate::db_model::get_block_by_hash(pool, from_block.clone())
		.await?
		.ok_or(DataSourceError::ExpectedDataNotFound(format!(
			"Lower bound block {from_block} not found when querying for native token transfers"
		)))?
		.block_no)
}

async fn get_to_block_no(to_block: &McBlockHash, pool: &PgPool) -> Result<BlockNumber> {
	Ok(crate::db_model::get_block_by_hash(pool, to_block.clone())
		.await?
		.ok_or(DataSourceError::ExpectedDataNotFound(format!(
			"Upper bound block {to_block} not found when querying for native token transfers"
		)))?
		.block_no)
}

async fn get_latest_block(pool: &PgPool) -> Result<Block> {
	crate::db_model::get_latest_block_info(pool).await?.ok_or(
		DataSourceError::ExpectedDataNotFound(
			"The latest block not found when querying for native token transfers".to_string(),
		),
	)
}

#[derive(Default)]
pub(crate) struct Cache {
	/// Continous blocks with their respective total native token transfer amount
	block_hash_to_amount: Vec<(McBlockHash, u128)>,
	pub(crate) scripts: Option<MainChainScripts>,
}

impl Cache {
	/// Returns the sum of native token transfer amounts after `after` and to `to` block
	/// Returns None if `after` or `to` block is not found, because it indicates that the cache
	/// doesn't contain the required blocks interval.
	fn get_sum_in_range(&self, after: &McBlockHash, to: &McBlockHash) -> Option<u128> {
		let after_idx = self.block_hash_to_amount.iter().position(|(block, _)| block == after)?;
		let to_idx = self.block_hash_to_amount.iter().position(|(block, _)| block == to)?;
		let after_to = self.block_hash_to_amount.get((after_idx + 1)..=to_idx)?;
		Some(after_to.iter().map(|(_, amount)| amount).sum())
	}

	pub fn update(
		&mut self,
		block_hash_to_amount: Vec<(McBlockHash, u128)>,
		scripts: MainChainScripts,
	) {
		self.block_hash_to_amount = block_hash_to_amount;
		self.scripts = Some(scripts);
	}
}
