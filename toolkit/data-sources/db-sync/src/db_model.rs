use crate::db_datum::DbDatum;
use crate::{DataSourceError, SqlxError};
use bigdecimal::ToPrimitive;
use cardano_serialization_lib::PlutusData;
use chrono::NaiveDateTime;
pub use db_sync_sqlx::*;
use log::info;
use sidechain_domain::{
	MainchainBlock, McBlockHash, McBlockNumber, McEpochNumber, McSlotNumber, McTxHash, UtxoId,
	UtxoIndex,
};
use sqlx::{
	Decode, PgPool, Pool, Postgres, database::Database, error::BoxDynError, postgres::PgTypeInfo,
};
use std::{cell::OnceCell, str::FromStr, sync::Arc};
use tokio::sync::Mutex;

/// Db-Sync `tx_in.value` configuration field
#[derive(Debug, PartialEq, Copy, Clone)]
pub(crate) enum TxInConfiguration {
	/// Transaction inputs are linked using `tx_in` table
	Enabled,
	/// Transaction inputs are linked using `consumed_by_tx_id` column in `tx_out` table
	Consumed,
}

impl TxInConfiguration {
	pub(crate) async fn from_connection(pool: &Pool<Postgres>) -> Result<Self, SqlxError> {
		let tx_in_exists = sqlx::query_scalar::<_, i64>(
			"select count(*) from information_schema.tables where table_name = 'tx_in';",
		)
		.fetch_one(pool)
		.await? == 1;

		if !tx_in_exists {
			return Ok(Self::Consumed);
		}

		let tx_in_populated = sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM tx_in);")
			.fetch_one(pool)
			.await?;

		if tx_in_populated {
			return Ok(Self::Enabled);
		}

		Ok(Self::Consumed)
	}
}

/// Structure that queries, caches and provides Db-Sync configuration
pub struct DbSyncConfigurationProvider {
	/// Postgres connection pool
	pub(crate) pool: PgPool,
	/// Transaction input configuration used by Db-Sync
	pub(crate) tx_in_config: Arc<Mutex<OnceCell<TxInConfiguration>>>,
}

impl DbSyncConfigurationProvider {
	pub(crate) fn new(pool: PgPool) -> Self {
		Self { tx_in_config: Arc::new(Mutex::new(OnceCell::new())), pool }
	}

	pub(crate) async fn get_tx_in_config(
		&self,
	) -> std::result::Result<TxInConfiguration, DataSourceError> {
		let lock = self.tx_in_config.lock().await;
		if let Some(tx_in_config) = lock.get() {
			return Ok(*tx_in_config);
		} else {
			let tx_in_config = TxInConfiguration::from_connection(&self.pool).await?;
			lock.set(tx_in_config).map_err(|_| {
				DataSourceError::InternalDataSourceError(
					"Failed to set tx_in_config in DbSyncConfigurationProvider".into(),
				)
			})?;
			log::info!("Using configuration: {tx_in_config:?}");
			return Ok(tx_in_config);
		}
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct Block {
	pub block_no: BlockNumber,
	pub hash: [u8; 32],
	pub epoch_no: EpochNumber,
	pub slot_no: SlotNumber,
	pub time: NaiveDateTime,
}

#[cfg(feature = "block-source")]
impl From<Block> for MainchainBlock {
	fn from(b: Block) -> Self {
		MainchainBlock {
			number: McBlockNumber(b.block_no.0),
			hash: McBlockHash(b.hash),
			epoch: McEpochNumber(b.epoch_no.0),
			slot: McSlotNumber(b.slot_no.0),
			timestamp: b.time.and_utc().timestamp().try_into().expect("i64 timestamp is valid u64"),
		}
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct MainchainTxOutput {
	pub utxo_id: UtxoId,
	pub tx_block_no: BlockNumber,
	pub tx_slot_no: SlotNumber,
	pub tx_epoch_no: EpochNumber,
	pub tx_index_in_block: TxIndexInBlock,
	pub address: String,
	pub datum: Option<PlutusData>,
	pub tx_inputs: Vec<UtxoId>,
}

impl TryFrom<MainchainTxOutputRow> for MainchainTxOutput {
	type Error = sqlx::Error;
	fn try_from(r: MainchainTxOutputRow) -> Result<Self, Self::Error> {
		let tx_inputs: Result<Vec<UtxoId>, _> =
			r.tx_inputs.into_iter().map(|i| UtxoId::from_str(i.as_str())).collect();
		let tx_inputs = tx_inputs.map_err(|e| sqlx::Error::Decode(e.into()))?;
		Ok(MainchainTxOutput {
			utxo_id: UtxoId {
				tx_hash: McTxHash(r.utxo_id_tx_hash),
				index: UtxoIndex(r.utxo_id_index.0),
			},
			tx_block_no: r.tx_block_no,
			tx_slot_no: r.tx_slot_no,
			tx_epoch_no: r.tx_epoch_no,
			tx_index_in_block: r.tx_index_in_block,
			address: r.address,
			datum: r.datum.map(|d| d.0),
			tx_inputs,
		})
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct MainchainTxOutputRow {
	pub utxo_id_tx_hash: [u8; 32],
	pub utxo_id_index: TxIndex,
	pub tx_block_no: BlockNumber,
	pub tx_slot_no: SlotNumber,
	pub tx_epoch_no: EpochNumber,
	pub tx_index_in_block: TxIndexInBlock,
	pub address: String,
	pub datum: Option<DbDatum>,
	pub tx_inputs: Vec<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct StakePoolEntry {
	pub pool_hash: [u8; 28],
	pub stake: StakeDelegation,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct TokenTxOutput {
	pub origin_tx_hash: [u8; 32],
	pub utxo_index: TxIndex,
	pub tx_epoch_no: EpochNumber,
	pub tx_block_no: BlockNumber,
	pub tx_slot_no: SlotNumber,
	pub tx_block_index: TxIndexInBlock,
	pub datum: Option<DbDatum>,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct DatumOutput {
	pub datum: DbDatum,
}

#[derive(Debug, Clone, PartialEq, sqlx::Type)]
#[repr(i32)]
/// Describes the type of a single change to the governed map
pub(crate) enum GovernedMapAction {
	/// Spending of a governed map utxo
	Spend,
	/// Creation of a governed map utxo
	Create,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct DatumChangeOutput {
	pub datum: DbDatum,
	pub block_no: BlockNumber,
	pub block_index: TxIndexInBlock,
	pub action: GovernedMapAction,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NativeTokenAmount(pub u128);
impl From<NativeTokenAmount> for sidechain_domain::NativeTokenAmount {
	fn from(value: NativeTokenAmount) -> Self {
		Self(value.0)
	}
}

impl sqlx::Type<Postgres> for NativeTokenAmount {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_name("NUMERIC")
	}
}

impl<'r> Decode<'r, Postgres> for NativeTokenAmount {
	fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let decoded = <sqlx::types::BigDecimal as Decode<Postgres>>::decode(value)?;
		let i = decoded.to_u128().ok_or("NativeTokenQuantity is always a u128".to_string())?;
		Ok(Self(i))
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct BlockTokenAmount {
	pub block_hash: [u8; 32],
	pub amount: NativeTokenAmount,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct StakePoolDelegationOutputRow {
	pub epoch_stake_amount: StakeDelegation,
	pub pool_hash_raw: [u8; 28],
	pub stake_address_hash_raw: [u8; 29],
	pub stake_address_script_hash: Option<[u8; 28]>,
}

pub(crate) async fn get_stake_pool_delegations_for_pools(
	pool: &Pool<Postgres>,
	epoch: EpochNumber,
	stake_pool_hashes: Vec<[u8; 28]>,
) -> Result<Vec<StakePoolDelegationOutputRow>, SqlxError> {
	Ok(sqlx::query_as::<_, StakePoolDelegationOutputRow>(
		"
SELECT
	epoch_stake.amount AS epoch_stake_amount,
	pool_hash.hash_raw AS pool_hash_raw,
	stake_address.hash_raw AS stake_address_hash_raw,
	stake_address.script_hash AS stake_address_script_hash
FROM
			   epoch_stake
	INNER JOIN stake_address      ON epoch_stake.addr_id = stake_address.id
	INNER JOIN pool_hash          ON epoch_stake.pool_id = pool_hash.id
WHERE
	    epoch_stake.epoch_no = $1
	AND epoch_stake.amount > 0
	AND pool_hash.hash_raw IN (SELECT unnest($2))
    ",
	)
	.bind(epoch)
	.bind(stake_pool_hashes)
	.fetch_all(pool)
	.await?)
}

#[cfg(any(feature = "block-source", feature = "native-token"))]
pub(crate) async fn get_latest_block_info(
	pool: &Pool<Postgres>,
) -> Result<Option<Block>, SqlxError> {
	Ok(sqlx::query_as::<_, Block>(
		"
SELECT
  block.block_no,
  block.hash,
  block.epoch_no,
  block.slot_no,
  block.time
FROM
  block
WHERE block.block_no IS NOT NULL
ORDER BY block.block_no DESC
LIMIT 1
    ",
	)
	.fetch_optional(pool)
	.await?)
}

/// Query to get the block in the given range, ordered by block number
/// # Arguments
/// * `from` - the lowest included block number
/// * `to` - the highest included block number
#[cfg(feature = "block-source")]
pub(crate) async fn get_blocks_by_numbers(
	pool: &Pool<Postgres>,
	from: BlockNumber,
	to: BlockNumber,
) -> Result<Vec<Block>, SqlxError> {
	Ok(sqlx::query_as::<_, Block>(
		"
SELECT block_no, hash, epoch_no, slot_no, time
FROM block
WHERE block_no >= $1 AND block_no <= $2
ORDER BY block_no ASC",
	)
	.bind(from)
	.bind(to)
	.fetch_all(pool)
	.await?)
}

/// Query to get the highest block in the given time range, not higher than a given block
/// # Arguments
/// * `max_block_number` - block number upper boundary
/// * `min_time` - time lower boundary
/// * `min_slot` - slot lower boundary, for efficiency, because time is not indexed
/// * `max_time` - time upper boundary
/// * `max_slot` - slot upper boundary, for efficiency, because time is not indexed
#[cfg(feature = "block-source")]
pub(crate) async fn get_highest_block(
	pool: &Pool<Postgres>,
	max_block_number: BlockNumber,
	min_time: NaiveDateTime,
	min_slot: SlotNumber,
	max_time: NaiveDateTime,
	max_slot: SlotNumber,
) -> Result<Option<Block>, SqlxError> {
	Ok(sqlx::query_as::<_, Block>(
		"
SELECT block_no, hash, epoch_no, slot_no, time
FROM block
WHERE block_no <= $1 AND $2 <= slot_no AND slot_no <= $3 AND $4 <= time AND time <= $5
ORDER BY block_no DESC
LIMIT 1",
	)
	.bind(max_block_number)
	.bind(min_slot)
	.bind(max_slot)
	.bind(min_time)
	.bind(max_time)
	.fetch_optional(pool)
	.await?)
}

/// Query to get the block by its hash
#[cfg(any(feature = "block-source", feature = "native-token"))]
pub(crate) async fn get_block_by_hash(
	pool: &Pool<Postgres>,
	hash: McBlockHash,
) -> Result<Option<Block>, SqlxError> {
	Ok(sqlx::query_as::<_, Block>(
		"SELECT block_no, hash, epoch_no, slot_no, time FROM block WHERE hash = $1",
	)
	.bind(hash.0)
	.fetch_optional(pool)
	.await?)
}

#[cfg(feature = "candidate-source")]
pub(crate) async fn get_latest_block_for_epoch(
	pool: &Pool<Postgres>,
	epoch: EpochNumber,
) -> Result<Option<Block>, SqlxError> {
	// Query below contains additional filters for slot_no and block_no not null, because
	// there exists blocks in Byron Era with Ouroboros classic consensus that have null values for these fields.
	let sql = "SELECT block.block_no, block.hash, block.epoch_no, block.slot_no, block.time
		FROM block
		WHERE block.epoch_no <= $1 AND block.slot_no IS NOT NULL AND block.block_no IS NOT NULL
		ORDER BY block.slot_no DESC
		LIMIT 1";
	Ok(sqlx::query_as::<_, Block>(sql).bind(epoch).fetch_optional(pool).await?)
}

#[cfg(feature = "candidate-source")]
/// Returns number of the latest epoch that is stable - no block in such an epoch can be rolled back.
/// The latest stable epoch is one less than epoch of the highest stable block (HSB),
/// because unstable part could be replaced with blocks sequence starting from the block that has
/// the same epoch as HSB.
pub(crate) async fn get_latest_stable_epoch(
	pool: &Pool<Postgres>,
	security_parameter: u32,
) -> Result<Option<EpochNumber>, SqlxError> {
	let sql = "SELECT stable_block.epoch_no - 1 as epoch_no
FROM block INNER JOIN block as stable_block ON block.block_no - $1 = stable_block.block_no
WHERE block.block_no IS NOT NULL
ORDER BY block.block_no DESC
LIMIT 1";
	#[allow(deprecated)]
	Ok(sqlx::query_as::<_, EpochNumberRow>(sql)
		.bind(BlockNumber(security_parameter))
		.fetch_optional(pool)
		.await?
		.map(EpochNumber::from))
}

#[cfg(feature = "candidate-source")]
pub(crate) async fn get_stake_distribution(
	pool: &Pool<Postgres>,
	epoch: EpochNumber,
) -> Result<Vec<StakePoolEntry>, SqlxError> {
	let sql = "
        SELECT ph.hash_raw as pool_hash, SUM(es.amount) as stake
        FROM epoch_stake es
        INNER JOIN pool_hash ph ON es.pool_id = ph.id
        WHERE es.epoch_no = $1
        GROUP BY ph.hash_raw";
	Ok(sqlx::query_as::<_, StakePoolEntry>(sql).bind(epoch).fetch_all(pool).await?)
}

/// Returns the token data of the given policy at the given slot.
#[cfg(feature = "candidate-source")]
pub(crate) async fn get_token_utxo_for_epoch(
	pool: &Pool<Postgres>,
	asset: &Asset,
	epoch: EpochNumber,
) -> Result<Option<TokenTxOutput>, SqlxError> {
	// In practice queried assets always have empty name.
	// However, it's important to keep multi_asset.name condition, to enable use of compound index on multi_asset policy and name.
	let sql = "SELECT
			origin_tx.hash        AS origin_tx_hash,
        	tx_out.index          AS utxo_index,
        	origin_block.epoch_no AS tx_epoch_no,
        	origin_block.block_no AS tx_block_no,
        	origin_block.slot_no  AS tx_slot_no,
        	origin_tx.block_index AS tx_block_index,
        	datum.value           AS datum
        FROM ma_tx_out
        INNER JOIN multi_asset          ON ma_tx_out.ident = multi_asset.id
        INNER JOIN tx_out               ON ma_tx_out.tx_out_id = tx_out.id
        INNER JOIN tx origin_tx         ON tx_out.tx_id = origin_tx.id
        INNER JOIN block origin_block   ON origin_tx.block_id = origin_block.id
        LEFT JOIN datum                 ON tx_out.data_hash = datum.hash
        WHERE multi_asset.policy = $1
		AND multi_asset.name = $2
        AND origin_block.epoch_no <= $3
        ORDER BY tx_block_no DESC, origin_tx.block_index DESC
        LIMIT 1";
	Ok(sqlx::query_as::<_, TokenTxOutput>(sql)
		.bind(&asset.policy_id.0)
		.bind(&asset.asset_name.0)
		.bind(epoch)
		.fetch_optional(pool)
		.await?)
}

#[cfg(feature = "governed-map")]
pub(crate) async fn get_governed_map_changes(
	pool: &Pool<Postgres>,
	address: &Address,
	after_block: Option<BlockNumber>,
	to_block: BlockNumber,
	asset: Asset,
	tx_in_configuration: TxInConfiguration,
) -> Result<Vec<DatumChangeOutput>, SqlxError> {
	match tx_in_configuration {
		TxInConfiguration::Enabled => {
			get_governed_map_changes_tx_in_enabled(pool, address, after_block, to_block, asset)
				.await
		},
		TxInConfiguration::Consumed => {
			get_governed_map_changes_tx_in_consumed(pool, address, after_block, to_block, asset)
				.await
		},
	}
}

#[cfg(feature = "governed-map")]
pub(crate) async fn get_governed_map_changes_tx_in_enabled(
	pool: &Pool<Postgres>,
	address: &Address,
	after_block: Option<BlockNumber>,
	to_block: BlockNumber,
	asset: Asset,
) -> Result<Vec<DatumChangeOutput>, SqlxError> {
	let query = "
		((SELECT
			datum.value as datum, origin_block.block_no as block_no, origin_tx.block_index as block_index, $6 as action, 1 as action_order
		FROM tx_out
		INNER JOIN tx origin_tx			ON tx_out.tx_id = origin_tx.id
		INNER JOIN block origin_block	ON origin_tx.block_id = origin_block.id
		INNER JOIN datum				ON tx_out.data_hash = datum.hash
		INNER JOIN ma_tx_out			ON tx_out.id = ma_tx_out.tx_out_id
		INNER JOIN multi_asset			ON multi_asset.id = ma_tx_out.ident
		WHERE
			tx_out.address = $1 AND ($2 IS NULL OR origin_block.block_no > $2) AND origin_block.block_no <= $3
			AND multi_asset.policy = $4
			AND multi_asset.name = $5)
		UNION
		(SELECT
			datum.value as datum, consuming_block.block_no as block_no, consuming_tx.block_index as block_index, $7 as action, -1 as action_order
		FROM tx_out
		LEFT JOIN tx_in consuming_tx_in	ON tx_out.tx_id = consuming_tx_in.tx_out_id AND tx_out.index = consuming_tx_in.tx_out_index
		LEFT JOIN tx consuming_tx		ON consuming_tx_in.tx_in_id = consuming_tx.id
		LEFT JOIN block consuming_block	ON consuming_tx.block_id = consuming_block.id
		INNER JOIN datum				ON tx_out.data_hash = datum.hash
		INNER JOIN ma_tx_out			ON tx_out.id = ma_tx_out.tx_out_id
		INNER JOIN multi_asset			ON multi_asset.id = ma_tx_out.ident
		WHERE
			tx_out.address = $1
			AND (consuming_tx_in.id IS NOT NULL AND ($2 IS NULL OR consuming_block.block_no > $2) AND consuming_block.block_no <= $3)
			AND multi_asset.policy = $4
			AND multi_asset.name = $5))
		ORDER BY block_no, block_index, action_order ASC";
	Ok(sqlx::query_as::<_, DatumChangeOutput>(query)
		.bind(&address.0)
		.bind(after_block)
		.bind(to_block)
		.bind(&asset.policy_id.0)
		.bind(&asset.asset_name.0)
		.bind(GovernedMapAction::Create)
		.bind(GovernedMapAction::Spend)
		.fetch_all(pool)
		.await?)
}

#[cfg(feature = "governed-map")]
pub(crate) async fn get_governed_map_changes_tx_in_consumed(
	pool: &Pool<Postgres>,
	address: &Address,
	after_block: Option<BlockNumber>,
	to_block: BlockNumber,
	asset: Asset,
) -> Result<Vec<DatumChangeOutput>, SqlxError> {
	let query = "
		((SELECT
			datum.value as datum, origin_block.block_no as block_no, origin_tx.block_index as block_index, $6 as action, 1 as action_order
		FROM tx_out
		INNER JOIN tx origin_tx			ON tx_out.tx_id = origin_tx.id
		INNER JOIN block origin_block	ON origin_tx.block_id = origin_block.id
		INNER JOIN datum				ON tx_out.data_hash = datum.hash
		INNER JOIN ma_tx_out			ON tx_out.id = ma_tx_out.tx_out_id
		INNER JOIN multi_asset			ON multi_asset.id = ma_tx_out.ident
		WHERE
			tx_out.address = $1 AND ($2 IS NULL OR origin_block.block_no > $2) AND origin_block.block_no <= $3
			AND multi_asset.policy = $4
			AND multi_asset.name = $5)
		UNION
		(SELECT
			datum.value as datum, consuming_block.block_no as block_no, consuming_tx.block_index as block_index, $7 as action, -1 as action_order
		FROM tx_out
		LEFT JOIN tx consuming_tx		ON tx_out.consumed_by_tx_id = consuming_tx.id
		LEFT JOIN block consuming_block	ON consuming_tx.block_id = consuming_block.id
		INNER JOIN datum				ON tx_out.data_hash = datum.hash
		INNER JOIN ma_tx_out			ON tx_out.id = ma_tx_out.tx_out_id
		INNER JOIN multi_asset			ON multi_asset.id = ma_tx_out.ident
		WHERE
			tx_out.address = $1
			AND (tx_out.consumed_by_tx_id IS NOT NULL AND ($2 IS NULL OR consuming_block.block_no > $2) AND consuming_block.block_no <= $3)
			AND multi_asset.policy = $4
			AND multi_asset.name = $5))
		ORDER BY block_no, block_index, action_order ASC";
	Ok(sqlx::query_as::<_, DatumChangeOutput>(query)
		.bind(&address.0)
		.bind(after_block)
		.bind(to_block)
		.bind(&asset.policy_id.0)
		.bind(&asset.asset_name.0)
		.bind(GovernedMapAction::Create)
		.bind(GovernedMapAction::Spend)
		.fetch_all(pool)
		.await?)
}

#[cfg(feature = "governed-map")]
pub(crate) async fn get_datums_at_address_with_token(
	pool: &Pool<Postgres>,
	address: &Address,
	block: BlockNumber,
	asset: Asset,
	tx_in_configuration: TxInConfiguration,
) -> Result<Vec<DatumOutput>, SqlxError> {
	match tx_in_configuration {
		TxInConfiguration::Enabled => {
			get_datums_at_address_with_token_tx_in_enabled(pool, address, block, asset).await
		},
		TxInConfiguration::Consumed => {
			get_datums_at_address_with_token_tx_in_consumed(pool, address, block, asset).await
		},
	}
}

#[cfg(feature = "governed-map")]
pub(crate) async fn get_datums_at_address_with_token_tx_in_enabled(
	pool: &Pool<Postgres>,
	address: &Address,
	block: BlockNumber,
	asset: Asset,
) -> Result<Vec<DatumOutput>, SqlxError> {
	let query = "
			SELECT
				datum.value as datum
			FROM tx_out
			INNER JOIN tx origin_tx			ON tx_out.tx_id = origin_tx.id
			INNER JOIN block origin_block	ON origin_tx.block_id = origin_block.id
			LEFT JOIN tx_in consuming_tx_in	ON tx_out.tx_id = consuming_tx_in.tx_out_id AND tx_out.index = consuming_tx_in.tx_out_index
			LEFT JOIN tx consuming_tx		ON consuming_tx_in.tx_in_id = consuming_tx.id
			LEFT JOIN block consuming_block	ON consuming_tx.block_id = consuming_block.id
			INNER JOIN datum				ON tx_out.data_hash = datum.hash
			INNER JOIN ma_tx_out			ON tx_out.id = ma_tx_out.tx_out_id
			INNER JOIN multi_asset			ON multi_asset.id = ma_tx_out.ident
			WHERE
				tx_out.address = $1 AND origin_block.block_no <= $2
				AND (consuming_tx_in.id IS NULL OR consuming_block.block_no > $2)
				AND multi_asset.policy = $3
				AND multi_asset.name = $4
				ORDER BY origin_block.block_no ASC, origin_tx.block_index ASC";
	Ok(sqlx::query_as::<_, DatumOutput>(query)
		.bind(&address.0)
		.bind(block)
		.bind(&asset.policy_id.0)
		.bind(&asset.asset_name.0)
		.fetch_all(pool)
		.await?)
}

#[cfg(feature = "governed-map")]
pub(crate) async fn get_datums_at_address_with_token_tx_in_consumed(
	pool: &Pool<Postgres>,
	address: &Address,
	block: BlockNumber,
	asset: Asset,
) -> Result<Vec<DatumOutput>, SqlxError> {
	let query = "
			SELECT
				datum.value as datum
			FROM tx_out
			INNER JOIN tx origin_tx			ON tx_out.tx_id = origin_tx.id
			INNER JOIN block origin_block	ON origin_tx.block_id = origin_block.id
			LEFT JOIN tx consuming_tx		ON tx_out.consumed_by_tx_id = consuming_tx.id
			LEFT JOIN block consuming_block	ON consuming_tx.block_id = consuming_block.id
			INNER JOIN datum				ON tx_out.data_hash = datum.hash
			INNER JOIN ma_tx_out			ON tx_out.id = ma_tx_out.tx_out_id
			INNER JOIN multi_asset			ON multi_asset.id = ma_tx_out.ident
			WHERE
				tx_out.address = $1 AND origin_block.block_no <= $2
				AND (tx_out.consumed_by_tx_id IS NULL OR consuming_block.block_no > $2)
				AND multi_asset.policy = $3
				AND multi_asset.name = $4
				ORDER BY origin_block.block_no ASC, origin_tx.block_index ASC";
	Ok(sqlx::query_as::<_, DatumOutput>(query)
		.bind(&address.0)
		.bind(block)
		.bind(&asset.policy_id.0)
		.bind(&asset.asset_name.0)
		.fetch_all(pool)
		.await?)
}

#[cfg(feature = "candidate-source")]
pub(crate) async fn get_epoch_nonce(
	pool: &Pool<Postgres>,
	epoch: EpochNumber,
) -> Result<Option<MainchainEpochNonce>, SqlxError> {
	let sql = "SELECT nonce FROM epoch_param WHERE epoch_no = $1";
	Ok(sqlx::query_as::<_, MainchainEpochNonce>(sql)
		.bind(epoch)
		.fetch_optional(pool)
		.await?)
}
#[cfg(feature = "candidate-source")]
pub(crate) async fn get_utxos_for_address(
	pool: &Pool<Postgres>,
	address: &Address,
	block: BlockNumber,
	tx_in_configuration: TxInConfiguration,
) -> Result<Vec<MainchainTxOutput>, SqlxError> {
	match tx_in_configuration {
		TxInConfiguration::Enabled => {
			get_utxos_for_address_tx_in_enabled(pool, address, block).await
		},
		TxInConfiguration::Consumed => {
			get_utxos_for_address_tx_in_consumed(pool, address, block).await
		},
	}
}

#[cfg(feature = "candidate-source")]
pub(crate) async fn get_utxos_for_address_tx_in_enabled(
	pool: &Pool<Postgres>,
	address: &Address,
	block: BlockNumber,
) -> Result<Vec<MainchainTxOutput>, SqlxError> {
	let query = "SELECT
          		origin_tx.hash as utxo_id_tx_hash,
          		tx_out.index as utxo_id_index,
          		origin_block.block_no as tx_block_no,
          		origin_block.slot_no as tx_slot_no,
          		origin_block.epoch_no as tx_epoch_no,
          		origin_tx.block_index as tx_index_in_block,
          		tx_out.address,
          		datum.value as datum,
          		array_agg(concat_ws('#', encode(consumes_tx.hash, 'hex'), consumes_tx_in.tx_out_index)) as tx_inputs
			FROM tx_out
			INNER JOIN tx    origin_tx       ON tx_out.tx_id = origin_tx.id
			INNER JOIN block origin_block    ON origin_tx.block_id = origin_block.id
          	LEFT JOIN tx_in consuming_tx_in  ON tx_out.tx_id = consuming_tx_in.tx_out_id AND tx_out.index = consuming_tx_in.tx_out_index
          	LEFT JOIN tx    consuming_tx     ON consuming_tx_in.tx_in_id = consuming_tx.id
          	LEFT JOIN block consuming_block  ON consuming_tx.block_id = consuming_block.id
			LEFT JOIN tx_in consumes_tx_in   ON consumes_tx_in.tx_in_id = origin_tx.id
          	LEFT JOIN tx_out consumes_tx_out ON consumes_tx_out.tx_id = consumes_tx_in.tx_out_id AND consumes_tx_in.tx_out_index = consumes_tx_out.index
          	LEFT JOIN tx consumes_tx         ON consumes_tx.id = consumes_tx_out.tx_id
          	LEFT JOIN datum                  ON tx_out.data_hash = datum.hash
          	WHERE
          		tx_out.address = $1 AND origin_block.block_no <= $2
          		AND (consuming_tx_in.id IS NULL OR consuming_block.block_no > $2)
          		GROUP BY (
					utxo_id_tx_hash,
					utxo_id_index,
					tx_block_no,
					tx_slot_no,
					tx_epoch_no,
					tx_index_in_block,
					tx_out.address,
					datum
				)";
	let rows = sqlx::query_as::<_, MainchainTxOutputRow>(query)
		.bind(&address.0)
		.bind(block)
		.fetch_all(pool)
		.await?;
	let result: Result<Vec<MainchainTxOutput>, sqlx::Error> =
		rows.into_iter().map(MainchainTxOutput::try_from).collect();
	Ok(result?)
}

#[cfg(feature = "candidate-source")]
pub(crate) async fn get_utxos_for_address_tx_in_consumed(
	pool: &Pool<Postgres>,
	address: &Address,
	block: BlockNumber,
) -> Result<Vec<MainchainTxOutput>, SqlxError> {
	let query = "SELECT
          		origin_tx.hash as utxo_id_tx_hash,
          		tx_out.index as utxo_id_index,
          		origin_block.block_no as tx_block_no,
          		origin_block.slot_no as tx_slot_no,
          		origin_block.epoch_no as tx_epoch_no,
          		origin_tx.block_index as tx_index_in_block,
          		tx_out.address,
          		datum.value as datum,
          		array_agg(concat_ws('#', encode(consumes_tx.hash, 'hex'), consumes_tx_out.index)) as tx_inputs
			FROM tx_out
			INNER JOIN tx    origin_tx       ON tx_out.tx_id = origin_tx.id
			INNER JOIN block origin_block    ON origin_tx.block_id = origin_block.id
			LEFT JOIN tx    consuming_tx     ON tx_out.consumed_by_tx_id = consuming_tx.id
			LEFT JOIN block consuming_block  ON consuming_tx.block_id = consuming_block.id
			LEFT JOIN tx_out consumes_tx_out ON consumes_tx_out.consumed_by_tx_id = origin_tx.id
			LEFT JOIN tx consumes_tx         ON consumes_tx.id = consumes_tx_out.tx_id
			LEFT JOIN datum                  ON tx_out.data_hash = datum.hash
          	WHERE
          		tx_out.address = $1 AND origin_block.block_no <= $2
          		AND (tx_out.consumed_by_tx_id IS NULL OR consuming_block.block_no > $2)
          		GROUP BY (
					utxo_id_tx_hash,
					utxo_id_index,
					tx_block_no,
					tx_slot_no,
					tx_epoch_no,
					tx_index_in_block,
					tx_out.address,
					datum
				)";
	let rows = sqlx::query_as::<_, MainchainTxOutputRow>(query)
		.bind(&address.0)
		.bind(block)
		.fetch_all(pool)
		.await?;
	let result: Result<Vec<MainchainTxOutput>, sqlx::Error> =
		rows.into_iter().map(MainchainTxOutput::try_from).collect();
	Ok(result?)
}
/// Used by `get_token_utxo_for_epoch` (CandidatesDataSourceImpl),
#[cfg(feature = "candidate-source")]
pub(crate) async fn create_idx_ma_tx_out_ident(pool: &Pool<Postgres>) -> Result<(), SqlxError> {
	let exists = index_exists(pool, "idx_ma_tx_out_ident").await?;
	if exists {
		info!("Index 'idx_ma_tx_out_ident' already exists");
	} else {
		let sql = "CREATE INDEX IF NOT EXISTS idx_ma_tx_out_ident ON ma_tx_out(ident)";
		info!("Executing '{}', this might take a while", sql);
		sqlx::query(sql).execute(pool).await?;
		info!("Index 'idx_ma_tx_out_ident' has been created");
	}
	Ok(())
}

/// Used by multiple queries across functionalities.
#[cfg(any(feature = "candidate-source", feature = "native-token", feature = "governed-map"))]
pub(crate) async fn create_idx_tx_out_address(pool: &Pool<Postgres>) -> Result<(), SqlxError> {
	let exists = index_exists(pool, "idx_tx_out_address").await?;
	if exists {
		info!("Index 'idx_tx_out_address' already exists");
	} else {
		let sql = "CREATE INDEX IF NOT EXISTS idx_tx_out_address ON tx_out USING hash (address)";
		info!("Executing '{}', this might take a long time", sql);
		sqlx::query(sql).execute(pool).await?;
		info!("Index 'idx_tx_out_address' has been created");
	}
	Ok(())
}

/// Check if the index exists.
async fn index_exists(pool: &Pool<Postgres>, index_name: &str) -> Result<bool, sqlx::Error> {
	sqlx::query("select * from pg_indexes where indexname = $1")
		.bind(index_name)
		.fetch_all(pool)
		.await
		.map(|rows| rows.len() == 1)
}

#[cfg(test)]
/// Check if the index exists. Panics on errors.
pub(crate) async fn index_exists_unsafe(pool: &Pool<Postgres>, index_name: &str) -> bool {
	index_exists(pool, index_name).await.unwrap()
}

/// Sums all transfers between genesis and the first block that is produced with the feature on.
/// Used by `get_native_token_transfers` (NativeTokenDataSourceImpl).
#[cfg(feature = "native-token")]
pub(crate) async fn get_total_native_tokens_transfered(
	pool: &Pool<Postgres>,
	to_block: BlockNumber,
	asset: Asset,
	illiquid_supply_address: Address,
) -> Result<NativeTokenAmount, SqlxError> {
	let query = sqlx::query_as::<_, (NativeTokenAmount,)>(
		"
SELECT
    COALESCE(SUM(ma_tx_out.quantity), 0)
FROM tx_out
LEFT JOIN ma_tx_out    ON ma_tx_out.tx_out_id = tx_out.id
LEFT JOIN multi_asset  ON multi_asset.id = ma_tx_out.ident
INNER JOIN tx          ON tx_out.tx_id = tx.id
INNER JOIN block       ON tx.block_id = block.id
WHERE address = $1
AND multi_asset.policy = $2
AND multi_asset.name = $3
AND block.block_no <= $4;
    ",
	)
	.bind(&illiquid_supply_address.0)
	.bind(&asset.policy_id.0)
	.bind(&asset.asset_name.0)
	.bind(to_block);

	Ok(query.fetch_one(pool).await?.0)
}

/// Returns the list of all blocks in given range,
/// together with the sum of all token amounts transferred to illiquid supply address in this block.
/// On devnet postgres it takes around 5ms to execute when querying 1000 blocks.
/// Used by `get_native_token_transfers` (NativeTokenDataSourceImpl).
#[cfg(feature = "native-token")]
pub(crate) async fn get_native_token_transfers(
	pool: &Pool<Postgres>,
	from_block: BlockNumber,
	to_block: BlockNumber,
	asset: Asset,
	illiquid_supply_address: Address,
) -> Result<Vec<BlockTokenAmount>, SqlxError> {
	let query = sqlx::query_as::<_, BlockTokenAmount>(
		"
SELECT
	block.hash as block_hash, COALESCE(a.amount, 0) as amount
FROM block
	LEFT JOIN (SELECT tx.block_id as block_id, SUM(ma_tx_out.quantity) as amount FROM tx
		INNER JOIN tx_out ON tx.id = tx_out.tx_id AND tx_out.address = $1
		INNER JOIN ma_tx_out    ON ma_tx_out.tx_out_id = tx_out.id
		INNER JOIN multi_asset  ON multi_asset.id = ma_tx_out.ident
		WHERE multi_asset.policy = $2 AND multi_asset.name = $3
		GROUP BY tx.block_id
		) as a ON block.id = a.block_id
	WHERE
		$4 <= block.block_no AND block.block_no <= $5
	ORDER BY block.block_no ASC;
",
	)
	.bind(&illiquid_supply_address.0)
	.bind(&asset.policy_id.0)
	.bind(&asset.asset_name.0)
	.bind(from_block)
	.bind(to_block);

	Ok(query.fetch_all(pool).await?)
}

#[cfg(test)]
mod tests {
	use sqlx::PgPool;

	use super::TxInConfiguration;

	#[sqlx::test(migrations = "./testdata/migrations-tx-in-enabled")]
	async fn tx_in_configuration_is_enabled_if_tx_in_table_exists(pool: PgPool) {
		let tx_in_config = TxInConfiguration::from_connection(&pool).await.unwrap();

		assert_eq!(tx_in_config, TxInConfiguration::Enabled)
	}

	#[sqlx::test(migrations = false)]
	async fn tx_in_configuration_is_consumed_if_tx_in_table_does_not_exist(pool: PgPool) {
		let tx_in_config = TxInConfiguration::from_connection(&pool).await.unwrap();

		assert_eq!(tx_in_config, TxInConfiguration::Consumed)
	}
}
