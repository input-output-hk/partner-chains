use cardano_serialization_lib::{
	MetadataJsonSchema, TransactionMetadatum, encode_json_value_to_metadatum,
};
use sqlx::postgres::PgTypeInfo;
use sqlx::postgres::types::Oid;
use sqlx::types::JsonValue;
use sqlx::{Database, Decode, Postgres, error::BoxDynError};

/// Wraps TransactionMetadatum to provide sqlx::Decode and sqlx::Type implementations
#[derive(Debug, Clone, PartialEq)]
pub struct DbMetadatum(pub TransactionMetadatum);

impl sqlx::Type<Postgres> for DbMetadatum {
	fn type_info() -> <Postgres as sqlx::Database>::TypeInfo {
		PgTypeInfo::with_oid(Oid(3802))
	}
}

impl<'r> sqlx::Decode<'r, Postgres> for DbMetadatum {
	fn decode(value: <Postgres as Database>::ValueRef<'r>) -> Result<Self, BoxDynError> {
		let value: JsonValue = <JsonValue as Decode<Postgres>>::decode(value)?;
		let datum = encode_json_value_to_metadatum(value, MetadataJsonSchema::BasicConversions);
		Ok(DbMetadatum(datum?))
	}
}
