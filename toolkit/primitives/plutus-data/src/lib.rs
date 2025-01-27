use cardano_serialization_lib::{PlutusData, PlutusList};

pub mod d_param;
pub mod permissioned_candidates;
pub mod registered_candidates;
pub mod reserve;
pub mod version_oracle;

#[derive(Debug, PartialEq, thiserror::Error)]
#[error("Could not decode {datum:?} to {to}: {msg}")]
pub struct DataDecodingError {
	datum: PlutusData,
	to: String,
	msg: String,
}

type DecodingResult<T> = std::result::Result<T, DataDecodingError>;

pub trait PlutusDataExtensions {
	fn as_u64(&self) -> Option<u64>;
	fn as_u32(&self) -> Option<u32>;
	fn as_u16(&self) -> Option<u16>;
}

impl PlutusDataExtensions for cardano_serialization_lib::PlutusData {
	fn as_u64(&self) -> Option<u64> {
		self.as_integer()?.as_u64().map(u64::from)
	}
	fn as_u32(&self) -> Option<u32> {
		u32::try_from(self.as_integer()?.as_u64()?).ok()
	}
	fn as_u16(&self) -> Option<u16> {
		u16::try_from(self.as_u32()?).ok()
	}
}

/// Trait that provides decoding of versioned generic plutus data.
///
/// Versioned generic plutus data contain a version number and two data sections:
/// - datum - the data with stable schema, read and validated by smart contracts
/// - appendix - generic data with evolving schema indicated by the version number, not used by smart contracts
///
/// The corresponding definition in the smart contracts repo is:
/// ```haskell
/// data VersionedGenericDatum a = VersionedGenericDatum
///     { datum :: a
///     , genericData :: BuiltinData
///     , version :: Integer
///     }
/// ```
pub(crate) trait VersionedDatum: Sized {
	const NAME: &str;

	/// Parses versioned plutus data.
	fn decode(data: &PlutusData) -> DecodingResult<Self>;
}

/// Trait that provides decoding of verioned generic plutus data with a legacy schema support.
///
/// It is assumed that versions 0 and legacy are equivalent.
pub(crate) trait VersionedDatumWithLegacy: Sized {
	const NAME: &str;

	/// Parses plutus data schema that was used before datum versioning was added. Kept for backwards compatibility.
	fn decode_legacy(data: &PlutusData) -> Result<Self, String>;

	/// Parses versioned plutus data.
	///
	/// Parameters:
	/// * `version` - version number
	/// * `datum` - datum with schema specified by smart-contract
	/// * `appendix` - generic data ignored by smart-contract logic, schema is version dependent
	fn decode_versioned(
		version: u64,
		datum: &PlutusData,
		appendix: &PlutusData,
	) -> Result<Self, String>;
}

impl<T: VersionedDatumWithLegacy> VersionedDatum for T {
	const NAME: &str = <Self as VersionedDatumWithLegacy>::NAME;

	fn decode(data: &PlutusData) -> DecodingResult<Self> {
		(match plutus_data_version_and_payload(data) {
			None => Self::decode_legacy(data),
			Some(VersionedGenericDatum { datum, appendix, version }) => {
				Self::decode_versioned(version, &datum, &appendix)
			},
		})
		.map_err(|msg| decoding_error_and_log(data, Self::NAME, &msg))
	}
}

fn plutus_data_version_and_payload(data: &PlutusData) -> Option<VersionedGenericDatum> {
	let fields = data.as_list().filter(|outer_list| outer_list.len() == 3)?;

	Some(VersionedGenericDatum {
		datum: fields.get(0),
		appendix: fields.get(1),
		version: fields.get(2).as_u64()?,
	})
}

fn decoding_error_and_log(data: &PlutusData, to: &str, msg: &str) -> DataDecodingError {
	log::error!("Could not decode {data:?} to {to}: {msg}");
	DataDecodingError { datum: data.clone(), to: to.to_string(), msg: msg.to_string() }
}

/// This struct has the same shape as `VersionedGenericDatum` from smart-contracts.
/// It is used to help implementing a proper `From` trait for `PlutusData` for
/// datum types.
pub(crate) struct VersionedGenericDatum {
	pub datum: PlutusData,
	pub appendix: PlutusData,
	pub version: u64,
}

impl From<VersionedGenericDatum> for PlutusData {
	fn from(value: VersionedGenericDatum) -> Self {
		let mut list = PlutusList::new();
		list.add(&value.datum);
		list.add(&value.appendix);
		list.add(&PlutusData::new_integer(&value.version.into()));
		PlutusData::new_list(&list)
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

	pub(crate) fn json_to_plutus_data(
		json: serde_json::Value,
	) -> cardano_serialization_lib::PlutusData {
		cardano_serialization_lib::encode_json_value_to_plutus_datum(
			json,
			cardano_serialization_lib::PlutusDatumSchema::DetailedSchema,
		)
		.expect("test data is valid")
	}

	pub(crate) fn plutus_data_to_json(
		data: cardano_serialization_lib::PlutusData,
	) -> serde_json::Value {
		cardano_serialization_lib::decode_plutus_datum_to_json_value(
			&data,
			cardano_serialization_lib::PlutusDatumSchema::DetailedSchema,
		)
		.expect("test data is valid")
	}
}
