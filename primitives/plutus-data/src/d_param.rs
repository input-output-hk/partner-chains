use crate::{DataDecodingError, DecodingResult, PlutusDataExtensions};
use cardano_serialization_lib::PlutusData;

pub enum DParamDatum {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0 { num_permissioned_candidates: u16, num_registered_candidates: u16 },
}

impl From<DParamDatum> for sidechain_domain::DParameter {
	fn from(datum: DParamDatum) -> Self {
		match datum {
			DParamDatum::V0 { num_permissioned_candidates, num_registered_candidates } => {
				Self { num_permissioned_candidates, num_registered_candidates }
			},
		}
	}
}

impl TryFrom<PlutusData> for DParamDatum {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		decode_legacy_d_parameter_datum(datum)
	}
}

/// Parses plutus data schema that was used before datum versioning was added. Kept for backwards compatibility.
fn decode_legacy_d_parameter_datum(datum: PlutusData) -> DecodingResult<DParamDatum> {
	let d_parameter = datum
		.as_list()
		.filter(|datum| datum.len() == 2)
		.and_then(|items| {
			Some(DParamDatum::V0 {
				num_permissioned_candidates: items.get(0).as_u16()?,
				num_registered_candidates: items.get(1).as_u16()?,
			})
		})
		.ok_or_else(|| {
			log::error!("Could not decode {:?} to DParameter. Expected [u16, u16].", datum.clone());
			DataDecodingError {
				datum: datum.clone(),
				to: "DParameter".to_string(),
				msg: "Expected [u16, u16]".to_string(),
			}
		})?;

	Ok(d_parameter)
}
