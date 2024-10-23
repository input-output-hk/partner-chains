use crate::DataSourceError::DatumDecodeError;
use cardano_serialization_lib::PlutusData;
use log::error;
use sidechain_domain::*;

use super::PlutusDataExtensions;

pub enum DParamDatum {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0 { num_permissioned_candidates: u16, num_registered_candidates: u16 },
}

impl TryFrom<PlutusData> for DParamDatum {
	type Error = super::Error;
	fn try_from(datum: PlutusData) -> super::Result<Self> {
		decode_legacy_d_parameter_datum(datum)
	}
}

impl From<DParamDatum> for DParameter {
	fn from(datum: DParamDatum) -> Self {
		match datum {
			DParamDatum::V0 { num_permissioned_candidates, num_registered_candidates } => {
				Self { num_permissioned_candidates, num_registered_candidates }
			},
		}
	}
}

fn decode_legacy_d_parameter_datum(datum: PlutusData) -> super::Result<DParamDatum> {
	let d_parameter = match datum.as_list() {
		Some(items) if items.len() == 2 => match (items.get(0).as_u16(), items.get(1).as_u16()) {
			(Some(p), Some(t)) => Some(DParamDatum::V0 {
				num_permissioned_candidates: p,
				num_registered_candidates: t,
			}),
			_ => None,
		},
		_ => None,
	}
	.ok_or(DatumDecodeError { datum: datum.clone(), to: "DParameter".to_string() });
	if d_parameter.is_err() {
		error!("Could not decode {:?} to DParameter. Expected [u16, u16].", datum.clone());
	}
	Ok(d_parameter?)
}
