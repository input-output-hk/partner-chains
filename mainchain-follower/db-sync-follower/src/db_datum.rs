use cardano_serialization_lib::PlutusData;
use sqlx::database::HasValueRef;
use sqlx::error::BoxDynError;
use sqlx::postgres::{types::Oid, PgTypeInfo};
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
	fn decode(value: <Postgres as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
		let value: JsonValue = <JsonValue as Decode<Postgres>>::decode(value)?;
		let datum = PlutusData::from_json(
			&value.to_string(),
			cardano_serialization_lib::PlutusDatumSchema::DetailedSchema,
		);
		Ok(DbDatum(datum?))
	}
}
