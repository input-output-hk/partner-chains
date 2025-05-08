use num_traits::ToPrimitive;
use sidechain_domain::*;
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::PgTypeInfo;
use sqlx::*;

/// Generates sqlx implementations for an unsigned wrapper of types that are signed.
/// We expect that values will have always 0 as the most significant bit.
/// For example TxIndex is in range of [0, 2^15-1], it will be u16 in domain,
/// but it requires encoding and decoding like i16.
/// See txindex, word31 and word63 types in db-sync schema definition.
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

#[derive(Debug, Copy, Ord, PartialOrd, Clone, PartialEq, Eq)]
pub struct BlockNumber(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", BlockNumber, McBlockNumber);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct EpochNumber(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", EpochNumber, McEpochNumber);

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SlotNumber(pub u64);
sqlx_implementations_for_wrapper!(i64, "INT8", SlotNumber, McSlotNumber);

/// CREATE DOMAIN txindex AS smallint CONSTRAINT txindex_check CHECK ((VALUE >= 0));
#[derive(Debug, Clone, PartialEq)]
pub struct TxIndex(pub u16);
sqlx_implementations_for_wrapper!(i16, "INT2", TxIndex, UtxoIndex);

#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
pub struct TxIndexInBlock(pub u32);
sqlx_implementations_for_wrapper!(i32, "INT4", TxIndexInBlock, McTxIndexInBlock);

/// CREATE DOMAIN int65type AS numeric (20, 0) CHECK (VALUE >= -18446744073709551615 AND VALUE <= 18446744073709551615);
#[derive(Debug, Clone, PartialEq)]
pub struct TxValue(pub i128);

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
pub struct StakeDelegation(pub u64);

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

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct AssetName(pub Vec<u8>);

impl From<sidechain_domain::AssetName> for AssetName {
	fn from(name: sidechain_domain::AssetName) -> Self {
		Self(name.0.to_vec())
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct PolicyId(pub Vec<u8>);

impl From<sidechain_domain::PolicyId> for PolicyId {
	fn from(id: sidechain_domain::PolicyId) -> Self {
		Self(id.0.to_vec())
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct Asset {
	pub policy_id: PolicyId,
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

#[derive(Debug, Copy, Clone, sqlx::FromRow, PartialEq)]
pub struct EpochNumberRow(pub EpochNumber);

impl From<EpochNumberRow> for EpochNumber {
	fn from(r: EpochNumberRow) -> Self {
		r.0
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct Address(pub String);

impl From<MainchainAddress> for Address {
	fn from(addr: MainchainAddress) -> Self {
		Self(addr.to_string())
	}
}

#[derive(Debug, Clone, sqlx::FromRow, PartialEq)]
pub struct MainchainEpochNonce(pub Vec<u8>);
