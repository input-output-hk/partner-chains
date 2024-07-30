use plutus::{Datum, FromJsonError};
use sqlx::database::HasValueRef;
use sqlx::error::BoxDynError;
use sqlx::postgres::{types::Oid, PgTypeInfo};
use sqlx::types::JsonValue;
use sqlx::{Decode, Postgres};
use std::fmt::{Display, Formatter};

/// Wraps plutus::Datum to provide sqlx::Decode and sqlx::Type implementations
#[derive(Debug, Clone, PartialEq)]
pub struct DbDatum(pub Datum);

impl sqlx::Type<Postgres> for DbDatum {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_oid(Oid(3802))
	}
}

impl<'r> sqlx::Decode<'r, Postgres> for DbDatum
where
	JsonValue: Decode<'r, Postgres>,
{
	fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
		let value: JsonValue = <JsonValue as Decode<Postgres>>::decode(value)?;
		let datum: Datum = TryFrom::try_from(&value).map_err(DbDatumDecodeError)?;
		Ok(DbDatum(datum))
	}
}

#[derive(Clone, Debug)]
pub struct DbDatumDecodeError(pub FromJsonError);

impl Display for DbDatumDecodeError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}", self.0)
	}
}

impl std::error::Error for DbDatumDecodeError {}
