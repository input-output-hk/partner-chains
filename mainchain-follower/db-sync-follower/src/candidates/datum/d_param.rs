use crate::DataSourceError::DatumDecodeError;
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
