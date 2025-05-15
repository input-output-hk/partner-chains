use cardano_serialization_lib::{
	PlutusData, PlutusDatumSchema::DetailedSchema, encode_json_value_to_plutus_datum,
};
use sqlx::database::Database;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgTypeInfo, types::Oid};
use sqlx::types::JsonValue;
use sqlx::{Decode, Postgres};

/// Wraps PlutusData to provide sqlx::Decode and sqlx::Type implementations
#[derive(Debug, Clone, PartialEq)]
pub struct DbDatum(pub PlutusData);

impl sqlx::Type<Postgres> for DbDatum {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_oid(Oid(3802))
	}
}

impl<'r> sqlx::Decode<'r, Postgres> for DbDatum
where
	JsonValue: Decode<'r, Postgres>,
{
	fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let value: JsonValue = <JsonValue as Decode<Postgres>>::decode(value)?;
		let datum = encode_json_value_to_plutus_datum(value, DetailedSchema);
		Ok(DbDatum(datum?))
	}
}
