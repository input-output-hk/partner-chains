use async_trait::async_trait;
use blockfrost_openapi::models::{
	address_transactions_content_inner::AddressTransactionsContentInner,
	address_utxo_content_inner::AddressUtxoContentInner,
	asset_addresses_inner::AssetAddressesInner, asset_transactions_inner::AssetTransactionsInner,
	block_content::BlockContent, epoch_param_content::EpochParamContent,
	epoch_stake_pool_content_inner::EpochStakePoolContentInner, genesis_content::GenesisContent,
	pool_history_inner::PoolHistoryInner, pool_list_extended_inner::PoolListExtendedInner,
	tx_content::TxContent, tx_content_utxo::TxContentUtxo,
};
use sidechain_domain::*;

use crate::DataSourceError;

/// Mainchain block id, either a block hash or a block number
pub enum McBlockId {
	/// Domain type Mainchain block hash
	McBlockHash(McBlockHash),
	/// Domain type Mainchain block number
	McBlockNumber(McBlockNumber),
	/// Mainchain block hash returned as string by Blockfrost API
	Raw(String),
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
/// Mini Blockfrost API interface
pub trait MiniBFApi {
	/// UTXOs of the address.
	async fn addresses_utxos(
		&self,
		address: MainchainAddress,
	) -> Result<Vec<AddressUtxoContentInner>, DataSourceError>;
	/// Transactions on the address.
	async fn addresses_transactions(
		&self,
		address: MainchainAddress,
	) -> Result<Vec<AddressTransactionsContentInner>, DataSourceError>;

	/// List of a specific asset transactions.
	async fn assets_transactions(
		&self,
		asset_id: AssetId,
	) -> Result<Vec<AssetTransactionsInner>, DataSourceError>;
	/// List of a addresses containing a specific asset.
	async fn assets_addresses(
		&self,
		asset_id: AssetId,
	) -> Result<Vec<AssetAddressesInner>, DataSourceError>;

	/// Return the latest block available to the backends, also known as the tip of the blockchain.
	async fn blocks_latest(&self) -> Result<BlockContent, DataSourceError>;
	/// Return the content of a requested block.
	async fn blocks_by_id(
		&self,
		id: impl Into<McBlockId> + Send,
	) -> Result<BlockContent, DataSourceError>;
	/// Return the content of a requested block for a specific slot.
	async fn blocks_slot(&self, slot_number: McSlotNumber)
	-> Result<BlockContent, DataSourceError>;
	/// Return the list of blocks following a specific block.
	async fn blocks_next(
		&self,
		hash: impl Into<McBlockId> + Send,
	) -> Result<Vec<BlockContent>, DataSourceError>;
	/// Return the transactions within the block.
	async fn blocks_txs(
		&self,
		id: impl Into<McBlockId> + Send,
	) -> Result<Vec<String>, DataSourceError>;

	/// Return the blocks minted for the epoch specified.
	async fn epochs_blocks(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<Vec<String>, DataSourceError>;
	/// Return the protocol parameters for the epoch specified.
	async fn epochs_parameters(
		&self,
		epoch_number: McEpochNumber,
	) -> Result<EpochParamContent, DataSourceError>;
	/// Return the active stake distribution for the epoch specified by stake pool.
	async fn epochs_stakes_by_pool(
		&self,
		epoch_number: McEpochNumber,
		pool_id: &str,
	) -> Result<Vec<EpochStakePoolContentInner>, DataSourceError>;

	/// History of stake pool parameters over epochs.
	async fn pools_history(&self, pool_id: &str) -> Result<Vec<PoolHistoryInner>, DataSourceError>;
	/// List of registered stake pools with additional information.
	async fn pools_extended(&self) -> Result<Vec<PoolListExtendedInner>, DataSourceError>;

	/// Query JSON value of a datum by its hash.
	async fn scripts_datum_hash(
		&self,
		datum_hash: &str,
	) -> Result<Vec<serde_json::Value>, DataSourceError>;

	/// Return content of the requested transaction.
	async fn transaction_by_hash(&self, tx_hash: McTxHash) -> Result<TxContent, DataSourceError>;
	/// Return the inputs and UTXOs of the specific transaction.
	async fn transactions_utxos(&self, tx_hash: McTxHash)
	-> Result<TxContentUtxo, DataSourceError>;

	/// Return the information about blockchain genesis.
	async fn genesis(&self) -> Result<GenesisContent, DataSourceError>;
}
