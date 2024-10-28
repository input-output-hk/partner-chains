use cardano_serialization_lib::PlutusData;

pub mod d_param;
pub mod permissioned_candidates;
pub mod registered_candidates;

#[derive(Debug, PartialEq, thiserror::Error)]
#[error("Could not decode {datum:?} to {to}: {msg}")]
pub struct DataDecodingError {
	datum: PlutusData,
	to: String,
	msg: String,
}

type DecodingResult<T> = std::result::Result<T, DataDecodingError>;

pub trait PlutusDataExtensions {
	fn as_u16(self) -> Option<u16>;
}

impl PlutusDataExtensions for cardano_serialization_lib::PlutusData {
	fn as_u16(self) -> Option<u16> {
		u16::try_from(u32::try_from(self.as_integer()?.as_u64()?).ok()?).ok()
	}
}

#[cfg(test)]
pub(crate) mod test_helpers {
	macro_rules! test_plutus_data {
		($json:tt) => {
			cardano_serialization_lib::encode_json_value_to_plutus_datum(
				serde_json::json!($json),
				cardano_serialization_lib::PlutusDatumSchema::DetailedSchema,
			)
			.expect("test data is valid")
		};
	}
	pub(crate) use test_plutus_data;
}
