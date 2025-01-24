use crate::{
	DataDecodingError, DecodingResult, PlutusDataExtensions, VersionedDatum,
	VersionedDatumWithLegacy, VersionedGenericDatum,
};
use cardano_serialization_lib::{PlutusData, PlutusList};

#[derive(Clone, Debug, PartialEq)]
pub enum DParamDatum {
	/// Initial/legacy datum schema. If a datum doesn't contain a version, it is assumed to be V0
	V0 { num_permissioned_candidates: u16, num_registered_candidates: u16 },
}

impl TryFrom<PlutusData> for DParamDatum {
	type Error = DataDecodingError;
	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		Self::decode(&datum)
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

pub fn d_parameter_to_plutus_data(d_param: &sidechain_domain::DParameter) -> PlutusData {
	let mut list = PlutusList::new();
	list.add(&PlutusData::new_integer(&d_param.num_permissioned_candidates.into()));
	list.add(&PlutusData::new_integer(&d_param.num_registered_candidates.into()));
	let generic_data = PlutusData::new_list(&list);
	VersionedGenericDatum {
		datum: PlutusData::new_empty_constr_plutus_data(&0u64.into()),
		generic_data,
		version: 0,
	}
	.into()
}

impl VersionedDatumWithLegacy for DParamDatum {
	const NAME: &str = "DParamDatum";

	fn decode_legacy(data: &PlutusData) -> Result<Self, String> {
		let d_parameter = data
			.as_list()
			.filter(|datum| datum.len() == 2)
			.and_then(|items| {
				Some(DParamDatum::V0 {
					num_permissioned_candidates: items.get(0).as_u16()?,
					num_registered_candidates: items.get(1).as_u16()?,
				})
			})
			.ok_or("Expected [u16, u16]")?;

		Ok(d_parameter)
	}

	fn decode_versioned(
		version: u64,
		_const_data: &PlutusData,
		mut_data: &PlutusData,
	) -> Result<Self, String> {
		match version {
			0 => DParamDatum::decode_legacy(mut_data)
				.map_err(|msg| format!("Can not parse mutable part of data: {msg}")),
			_ => Err(format!("Unknown version: {version}")),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_helpers::*;
	use pretty_assertions::assert_eq;

	#[test]
	fn valid_legacy_d_param() {
		let plutus_data = test_plutus_data!({"list": [{"int": 1}, {"int": 2}]});

		let expected_datum =
			DParamDatum::V0 { num_permissioned_candidates: 1, num_registered_candidates: 2 };

		assert_eq!(DParamDatum::try_from(plutus_data).unwrap(), expected_datum)
	}

	#[test]
	fn domain_d_param_to_csl() {
		let d_param = sidechain_domain::DParameter {
			num_permissioned_candidates: 17,
			num_registered_candidates: 42,
		};

		let expected_plutus_data = json_to_plutus_data(v0_datum_json());

		assert_eq!(d_parameter_to_plutus_data(&d_param), expected_plutus_data)
	}

	fn v0_datum_json() -> serde_json::Value {
		serde_json::json!({
			"list": [
				{ "constructor": 0, "fields": [] },
				{ "list": [
					{ "int": 17 },
					{ "int": 42 }
				] },
				{ "int": 0 }
			]
		})
	}

	#[test]
	fn valid_v0_d_param() {
		let plutus_data = json_to_plutus_data(v0_datum_json());

		let expected_datum =
			DParamDatum::V0 { num_permissioned_candidates: 17, num_registered_candidates: 42 };

		assert_eq!(DParamDatum::try_from(plutus_data).unwrap(), expected_datum)
	}
}
