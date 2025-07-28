//! helpers and primitive types for querying Cardano [Db-Sync] data using [sqlx]
//!
//! # About
//!
//! This crate is meant to provide help when writing queries for data produced by [Db-Sync],
//! a Cardano blockchain indexer. Db-Sync keeps the indexed data in a Postgre database that
//! can be queried using SQL. This crate defines primitive types that correspond to column
//! types used in the [Db-Sync schema], along with some other helpers and casts to types
//! defined in [sidechain_domain].
//!
//! [sqlx]: https://github.com/launchbadge/sqlx
//! [Db-Sync]: https://github.com/IntersectMBO/cardano-db-sync
//! [Db-Sync schema]: https://github.com/IntersectMBO/cardano-db-sync/blob/master/doc/schema.md
use num_traits::ToPrimitive;
use sidechain_domain::*;
use sqlx::database::Database;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::PgTypeInfo;
use sqlx::*;

/// Macro to handle numeric types that are non-negative but are stored by Db-Sync using
/// signed SQL types.
///
/// This macro generates an unsigned numeric domain type and sqlx trait implementation for
/// decoding it from signed data coming from Db-Sync database. It expects that values will
/// always have 0 as the most significant bit.
///
/// For example because [TxIndex] is in range of \[0, 2^15-1\], it will be [u16] in domain,
/// but it requires encoding and decoding as i16.
///
/// See `txindex`, `word31` `and` word63 types in Db-Sync schema definition.
#[macro_export]
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
			fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
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
				buf: &mut <Postgres as Database>::ArgumentBuffer<'q>,
			) -> std::result::Result<IsNull, BoxDynError> {
				buf.extend(&self.0.to_be_bytes());
				Ok(IsNull::No)
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

/// Cardano block number
#[derive(Debug, Copy, Ord, PartialOrd, Clone, PartialEq, Eq)]
pub struct BlockNumber(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", BlockNumber, McBlockNumber);

/// Cardano epoch number
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EpochNumber(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", EpochNumber, McEpochNumber);

/// Cardano slot number
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SlotNumber(pub u64);
sqlx_implementations_for_wrapper!(i64, "INT8", SlotNumber, McSlotNumber);

/// Index of a Cardano transaction output
///
/// This type corresponds to the following SQL type defined by Db-Sync.
/// ```sql
/// CREATE DOMAIN txindex AS smallint CONSTRAINT txindex_check CHECK ((VALUE >= 0));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TxIndex(pub u16);
sqlx_implementations_for_wrapper!(i16, "INT2", TxIndex, UtxoIndex);

/// Index of a Cardano transaction with its block
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct TxIndexInBlock(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", TxIndexInBlock, McTxIndexInBlock);

/// Number of ADA expressed in Lovelace (1 million-th of ADA)
///
/// This type corresponds to the following SQL type defined by Db-Sync.
/// ```sql
/// CREATE DOMAIN int65type AS numeric (20, 0) CHECK (VALUE >= -18446744073709551615 AND VALUE <= 18446744073709551615);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TxValue(pub i128);

impl sqlx::Type<Postgres> for TxValue {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_name("NUMERIC")
	}
}

impl<'r> Decode<'r, Postgres> for TxValue {
	fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let decoded = <sqlx::types::BigDecimal as Decode<Postgres>>::decode(value)?;
		let i = decoded.to_i128().ok_or("TxValue is always an integer".to_string())?;
		Ok(Self(i))
	}
}

/// Number of ADA delegated by a Cardano delegator to a single SPO, expressed in Lovelace (1 million-th of ADA)
///
/// This type corresponds to the following SQL type defined by Db-Sync.
/// ```sql
/// CREATE DOMAIN "lovelace" AS numeric(20,0) CONSTRAINT flyway_needs_this CHECK (VALUE >= 0::numeric AND VALUE <= '18446744073709551615'::numeric);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StakeDelegation(pub u64);

impl sqlx::Type<Postgres> for StakeDelegation {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_name("NUMERIC")
	}
}

impl<'r> Decode<'r, Postgres> for StakeDelegation {
	fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let decoded = <sqlx::types::BigDecimal as Decode<Postgres>>::decode(value)?;
		let i = decoded.to_u64().ok_or("StakeDelegation is always a u64".to_string())?;
		Ok(Self(i))
	}
}

/// Cardano native asset name, typically UTF-8 encoding of a human-readable name
#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct AssetName(pub Vec<u8>);

impl From<sidechain_domain::AssetName> for AssetName {
	fn from(name: sidechain_domain::AssetName) -> Self {
		Self(name.0.to_vec())
	}
}

/// Cardano minting policy ID. This value is obtained by hashing the Plutus script of the policy.
#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct PolicyId(pub Vec<u8>);

impl From<sidechain_domain::PolicyId> for PolicyId {
	fn from(id: sidechain_domain::PolicyId) -> Self {
		Self(id.0.to_vec())
	}
}

/// Full identifier of a Cardano native asset
#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct Asset {
	/// Minting policy ID of the asset
	pub policy_id: PolicyId,
	/// Asset name
	pub asset_name: AssetName,
}

impl Asset {
	/// Creates an Asset with empty asset_name.
	pub fn new(policy_id: sidechain_domain::PolicyId) -> Self {
		Self { policy_id: PolicyId(policy_id.0.to_vec()), asset_name: AssetName(vec![]) }
	}
}

impl From<sidechain_domain::AssetId> for Asset {
	fn from(asset: AssetId) -> Self {
		Self { policy_id: asset.policy_id.into(), asset_name: asset.asset_name.into() }
	}
}

/// Helper row type for querying just epoch number
#[allow(deprecated)]
mod epoch_number_row {
	use super::*;
	#[deprecated(
		since = "1.7.0",
		note = "Deprecated due to not being either a primitive type or a complete Db-Sync table row."
	)]
	#[derive(Debug, Copy, Clone, sqlx::FromRow, PartialEq)]
	pub struct EpochNumberRow(pub EpochNumber);

	#[allow(deprecated)]
	impl From<EpochNumberRow> for EpochNumber {
		fn from(r: EpochNumberRow) -> Self {
			r.0
		}
	}
}
pub use epoch_number_row::*;

/// Cardano address in human-readable form. Either Base58 for Byron addresses and Bech32 for Shelley.
#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct Address(pub String);

impl From<MainchainAddress> for Address {
	fn from(addr: MainchainAddress) -> Self {
		Self(addr.to_string())
	}
}

/// Cardano epoch nonce, ie. random 32 bytes generated by Cardano every epoch.
///
/// This value can be used as a tamper-proof randomness seed.
#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct MainchainEpochNonce(pub Vec<u8>);
