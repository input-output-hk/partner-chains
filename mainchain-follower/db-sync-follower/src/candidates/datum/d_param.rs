use crate::DataSourceError::DatumDecodeError;
use cardano_serialization_lib::PlutusData;
use log::error;
use num_traits::ToPrimitive;
use plutus::Datum::*;
use plutus::*;
use sidechain_domain::*;

pub enum DParamDatum {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0 { num_permissioned_candidates: u16, num_registered_candidates: u16 },
}

impl TryFrom<&Datum> for DParamDatum {
	type Error = super::Error;
	fn try_from(datum: &Datum) -> super::Result<Self> {
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

fn decode_legacy_d_parameter_datum(datum: &Datum) -> super::Result<DParamDatum> {
	let d_parameter = match datum {
		ListDatum(items) => match items.first().zip(items.get(1)) {
			Some((IntegerDatum(p), IntegerDatum(t))) => p.to_u16().zip(t.to_u16()).map(|(p, t)| {
				DParamDatum::V0 { num_permissioned_candidates: p, num_registered_candidates: t }
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

#[allow(dead_code)]
mod csl {
	use super::*;

	fn decode_legacy_d_parameter_datum(datum: &PlutusData) -> super::super::Result<DParamDatum> {
		let d_parameter = match datum.as_list() {
			Some(items) if items.len() == 2 => {
				match (items.get(0).as_integer(), items.get(1).as_integer()) {
					(Some(p), Some(t)) => {
						let p: Option<u16> = p
							.as_u64()
							.and_then(|v| u32::try_from(v).ok())
							.and_then(|v| u16::try_from(v).ok());
						let t: Option<u16> = t
							.as_u64()
							.and_then(|v| u32::try_from(v).ok())
							.and_then(|v| u16::try_from(v).ok());
						p.zip(t).map(|(p, t)| DParamDatum::V0 {
							num_permissioned_candidates: p,
							num_registered_candidates: t,
						})
					},
					_ => None,
				}
			},
			_ => None,
		}
		.ok_or(format!("error").into());
		// .ok_or(DatumDecodeError { datum: datum.clone(), to: "DParameter".to_string() });
		if d_parameter.is_err() {
			error!("Could not decode {:?} to DParameter. Expected [u16, u16].", datum.clone());
		}
		d_parameter
	}
}
