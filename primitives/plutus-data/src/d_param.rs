use crate::{DataDecodingError, DecodingResult, PlutusDataExtensions};
use cardano_serialization_lib::{PlutusData, PlutusList};

#[derive(Clone, Debug, PartialEq)]
pub enum DParamDatum {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0 { num_permissioned_candidates: u16, num_registered_candidates: u16 },
}

impl TryFrom<PlutusData> for DParamDatum {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		decode_legacy_d_parameter_datum(datum)
	}
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

impl From<sidechain_domain::DParameter> for DParamDatum {
	fn from(d_parameter: sidechain_domain::DParameter) -> Self {
		Self::V0 {
			num_permissioned_candidates: d_parameter.num_permissioned_candidates,
			num_registered_candidates: d_parameter.num_registered_candidates,
		}
	}
}

impl From<DParamDatum> for PlutusData {
	fn from(datum: DParamDatum) -> Self {
		match datum {
			DParamDatum::V0 { num_permissioned_candidates, num_registered_candidates } => {
				let mut list = PlutusList::new();
				list.add(&PlutusData::new_integer(&num_permissioned_candidates.into()));
				list.add(&PlutusData::new_integer(&num_registered_candidates.into()));
				PlutusData::new_list(&list)
			},
		}
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use cardano_serialization_lib::PlutusData;
	use pretty_assertions::assert_eq;

	#[test]
	fn valid_d_param_1() {
		let plutus_data = test_plutus_data!({"list": [{"int": 1}, {"int": 2}]});

		let expected_datum =
			DParamDatum::V0 { num_permissioned_candidates: 1, num_registered_candidates: 2 };

		assert_eq!(DParamDatum::try_from(plutus_data).unwrap(), expected_datum)
	}

	#[test]
	fn domain_d_param_to_csl() {
		let d_param = sidechain_domain::DParameter {
			num_permissioned_candidates: 1,
			num_registered_candidates: 2,
		};

		let expected_plutus_data = test_plutus_data!({"list": [{"int": 1}, {"int": 2}]});

		assert_eq!(PlutusData::from(DParamDatum::from(d_param)), expected_plutus_data)
	}
}
