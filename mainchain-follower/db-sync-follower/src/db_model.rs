use crate::db_datum::DbDatum;
use crate::SqlxError;
use chrono::NaiveDateTime;
use log::info;
use num_traits::ToPrimitive;
use plutus::Datum;
use sidechain_domain::*;
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::PgTypeInfo;
use sqlx::{Decode, Encode, Pool, Postgres};
use std::str::FromStr;

/// Generates sqlx implementations for an unsigned wrapper of types that are signed.
/// We expect that values will have always 0 as the most significant bit.
/// For example TxIndex is in range of [0, 2^15-1], it will be u16 in domain,
/// but it requires encoding and decoding like i16.
/// See txindex, word31 and word63 types in db-sync schema definition.
macro_rules! sqlx_implementations_for_wrapper {
	($WRAPPED:ty, $DBTYPE:expr, $NAME:ty, $DOMAIN:ty) => {
		impl sqlx::Type<Postgres> for $NAME {
			fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
				PgTypeInfo::with_name($DBTYPE)
			}
		}

		impl<'r> Decode<'r, Postgres> for $NAME
		where
			$WRAPPED: Decode<'r, Postgres>,
		{
			fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
				let decoded: $WRAPPED = <$WRAPPED as Decode<Postgres>>::decode(value)?;
				Ok(Self(decoded.try_into()?))
			}
		}

		#[cfg(test)]
		impl From<$WRAPPED> for $NAME {
			fn from(value: $WRAPPED) -> Self {
				Self(value.try_into().expect("value from domain fits in type db type"))
			}
		}

		impl<'q> Encode<'q, Postgres> for $NAME {
			fn encode_by_ref(
				&self,
				buf: &mut <Postgres as HasArguments<'q>>::ArgumentBuffer,
			) -> IsNull {
				buf.extend(&self.0.to_be_bytes());
				IsNull::No
			}
		}

		impl From<$NAME> for $DOMAIN {
			fn from(value: $NAME) -> Self {
				Self(value.0)
			}
		}

		impl From<$DOMAIN> for $NAME {
			fn from(value: $DOMAIN) -> Self {
				Self(value.0)
			}
		}
	};
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct Asset {
	pub policy_id: PolicyId,
	pub asset_name: AssetName,
}

impl Asset {
	/// Creates an Asset with empty asset_name.
	pub(crate) fn new(policy_id: sidechain_domain::PolicyId) -> Self {
		Self { policy_id: PolicyId(policy_id.0.to_vec()), asset_name: AssetName(vec![]) }
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct AssetName(pub Vec<u8>);

impl From<sidechain_domain::AssetName> for AssetName {
	fn from(name: sidechain_domain::AssetName) -> Self {
		Self(name.0.to_vec())
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct DistributedSetData {
	pub utxo_id_tx_hash: [u8; 32],
	pub utxo_id_index: TxIndex,
	pub asset_name: Vec<u8>,
	pub datum: DbDatum,
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

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct BlockNumber(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", BlockNumber, McBlockNumber);

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct EpochNumber(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", EpochNumber, McEpochNumber);

#[derive(Debug, Copy, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct EpochNumberRow(pub EpochNumber);

impl From<EpochNumberRow> for EpochNumber {
	fn from(r: EpochNumberRow) -> Self {
		r.0
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct Address(pub String);

impl From<MainchainAddress> for Address {
	fn from(addr: MainchainAddress) -> Self {
		Self(addr.to_string())
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct MainchainEpochNonce(pub Vec<u8>);

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct SlotNumber(pub u64);
sqlx_implementations_for_wrapper!(i64, "INT8", SlotNumber, McSlotNumber);

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct MainchainTxOutput {
	pub utxo_id: UtxoId,
	pub tx_block_no: BlockNumber,
	pub tx_slot_no: SlotNumber,
	pub tx_epoch_no: EpochNumber,
	pub tx_index_in_block: TxIndexInBlock,
	pub address: String,
	pub datum: Option<Datum>,
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
pub(crate) struct MintAction {
	pub tx_hash: [u8; 32],
	pub asset_name: Vec<u8>,
	pub block_no: BlockNumber,
	pub tx_index_in_block: i32,
	pub redeemer: Option<DbDatum>,
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct PolicyId(pub Vec<u8>);

impl From<sidechain_domain::PolicyId> for PolicyId {
	fn from(id: sidechain_domain::PolicyId) -> Self {
		Self(id.0.to_vec())
	}
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

/// CREATE DOMAIN txindex AS smallint CONSTRAINT txindex_check CHECK ((VALUE >= 0));
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TxIndex(pub u16);
sqlx_implementations_for_wrapper!(i16, "INT2", TxIndex, UtxoIndex);

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TxIndexInBlock(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", TxIndexInBlock, McTxIndexInBlock);

/// CREATE DOMAIN int65type AS numeric (20, 0) CHECK (VALUE >= -18446744073709551615 AND VALUE <= 18446744073709551615);
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TxValue(pub i128);

impl sqlx::Type<Postgres> for TxValue {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_name("NUMERIC")
	}
}

impl<'r> Decode<'r, Postgres> for TxValue {
	fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
		let decoded = <sqlx::types::BigDecimal as Decode<Postgres>>::decode(value)?;
		let i = decoded.to_i128().ok_or("TxValue is always an integer".to_string())?;
		Ok(Self(i))
	}
}

/// CREATE DOMAIN "lovelace" AS numeric(20,0) CONSTRAINT flyway_needs_this CHECK (VALUE >= 0::numeric AND VALUE <= '18446744073709551615'::numeric);
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StakeDelegation(pub u64);

impl sqlx::Type<Postgres> for StakeDelegation {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_name("NUMERIC")
	}
}

impl<'r> Decode<'r, Postgres> for StakeDelegation {
	fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
		let decoded = <sqlx::types::BigDecimal as Decode<Postgres>>::decode(value)?;
		let i = decoded.to_u64().ok_or("StakeDelegation is always a u64".to_string())?;
		Ok(Self(i))
	}
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
	fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
		let decoded = <sqlx::types::BigDecimal as Decode<Postgres>>::decode(value)?;
		let i = decoded.to_u128().ok_or("NativeTokenQuantity is always a u128".to_string())?;
		Ok(Self(i))
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct TxPosition {
	pub block_number: BlockNumber,
	pub index: TxIndexInBlock,
}

impl Default for TxPosition {
	fn default() -> Self {
		Self { block_number: BlockNumber(0), index: TxIndexInBlock(0) }
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub(crate) struct BlockTokenAmount {
	pub block_hash: [u8; 32],
	pub amount: NativeTokenAmount,
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
	let sql = "SELECT ph.hash_raw as pool_hash, SUM(es.amount) as stake FROM epoch_stake es
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
          		AND (consuming_tx_in.id IS NULL OR consuming_block.block_no > $3)
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
		.bind(block)
		.fetch_all(pool)
		.await?;
	let result: Result<Vec<MainchainTxOutput>, sqlx::Error> =
		rows.into_iter().map(MainchainTxOutput::try_from).collect();
	Ok(result?)
}
/// Used by `get_token_utxo_for_epoch` (CandidatesDataSourceImpl),
/// `get_distributed_set_utxo` (DistributedSetDataSourceImpl),
/// `get_nft_utxo` (CertificateDataSourceImpl),
pub(crate) async fn create_idx_ma_tx_out_ident(pool: &Pool<Postgres>) -> Result<(), SqlxError> {
	let sql = "CREATE INDEX IF NOT EXISTS idx_ma_tx_out_ident ON ma_tx_out(ident)";
	info!("Executing '{}', this might take a while", sql);
	sqlx::query(sql).execute(pool).await?;
	Ok(())
}

#[cfg(test)]
/// Check if the index exists.
pub(crate) async fn index_exists(pool: &Pool<Postgres>, index_name: &str) -> bool {
	sqlx::query("select * from pg_indexes where indexname = $1")
		.bind(index_name)
		.fetch_all(pool)
		.await
		.map(|rows| rows.len() == 1)
		.unwrap()
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
    block.hash as block_hash, COALESCE(SUM(ma_tx_out.quantity), 0) as amount
FROM block
    LEFT JOIN tx ON block.id = tx.block_id
    LEFT JOIN tx_out ON tx.id = tx_out.tx_id AND tx_out.address = $1
    LEFT JOIN ma_tx_out    ON ma_tx_out.tx_out_id = tx_out.id
    LEFT JOIN multi_asset  ON multi_asset.id = ma_tx_out.ident AND multi_asset.policy = $2 AND multi_asset.name = $3
WHERE
    $4 <= block.block_no AND block.block_no <= $5
GROUP BY block.block_no, block.hash
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
