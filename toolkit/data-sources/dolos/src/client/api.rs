use async_trait::async_trait;
use blockfrost_openapi::models::{
	address_transactions_content_inner::AddressTransactionsContentInner,
	address_utxo_content_inner::AddressUtxoContentInner,
	asset_addresses_inner::AssetAddressesInner, asset_transactions_inner::AssetTransactionsInner,
	block_content::BlockContent, epoch_param_content::EpochParamContent,
	epoch_stake_pool_content_inner::EpochStakePoolContentInner,
	pool_history_inner::PoolHistoryInner, pool_list_extended_inner::PoolListExtendedInner,
	tx_content::TxContent, tx_content_utxo::TxContentUtxo,
};
use sidechain_domain::*;

pub enum McBlockId {
	McBlockHash(McBlockHash),
	McBlockNumber(McBlockNumber),
	Raw(String), // makes you think
}

impl From<McBlockHash> for McBlockId {
	fn from(value: McBlockHash) -> Self {
		McBlockId::McBlockHash(value)
	}
}

impl From<McBlockNumber> for McBlockId {
	fn from(value: McBlockNumber) -> Self {
		McBlockId::McBlockNumber(value)
	}
}

impl From<String> for McBlockId {
	fn from(value: String) -> Self {
		McBlockId::Raw(value)
	}
}

impl std::fmt::Display for McBlockId {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			McBlockId::McBlockHash(mc_block_hash) => mc_block_hash.fmt(f),
			McBlockId::McBlockNumber(mc_block_number) => mc_block_number.fmt(f),
			McBlockId::Raw(str) => str.fmt(f),
		}
	}
}

#[async_trait]
pub trait MiniBFApi {
	async fn addresses_utxos(
		&self,
		address: MainchainAddress,
	) -> Result<Vec<AddressUtxoContentInner>, String>;
	async fn addresses_transactions(
		&self,
		address: MainchainAddress,
	) -> Result<Vec<AddressTransactionsContentInner>, String>;

	async fn assets_transactions(
		&self,
		asset_id: AssetId,
	) -> Result<Vec<AssetTransactionsInner>, String>;
	async fn assets_addresses(&self, asset_id: AssetId)
	-> Result<Vec<AssetAddressesInner>, String>;

	async fn blocks_latest(&self) -> Result<BlockContent, String>;
	async fn blocks_by_id(&self, id: impl Into<McBlockId> + Send) -> Result<BlockContent, String>;
	async fn blocks_slot(&self, slot_number: McSlotNumber) -> Result<BlockContent, String>;
	async fn blocks_next(
		&self,
		hash: impl Into<McBlockId> + Send,
	) -> Result<Vec<BlockContent>, String>;
	async fn blocks_txs(&self, id: impl Into<McBlockId> + Send) -> Result<Vec<String>, String>;

	async fn epochs_blocks(&self, epoch_number: McEpochNumber) -> Result<Vec<String>, String>;
	async fn epochs_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<EpochParamContent, String>;
	async fn epochs_stakes_by_pool(
		&self,
		epoch_number: McEpochNumber,
		pool_id: &str,
	) -> Result<Vec<EpochStakePoolContentInner>, String>;

	async fn pools_history(&self, pool_id: &str) -> Result<Vec<PoolHistoryInner>, String>;
	async fn pools_extended(&self) -> Result<Vec<PoolListExtendedInner>, String>;

	async fn scripts_datum_hash(&self, datum_hash: &str) -> Result<Vec<serde_json::Value>, String>;

	async fn transaction_by_hash(&self, tx_hash: McTxHash) -> Result<TxContent, String>;
	async fn transactions_utxos(&self, tx_hash: McTxHash) -> Result<TxContentUtxo, String>;
}
