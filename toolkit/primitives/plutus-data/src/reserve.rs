use crate::{
	decoding_error_and_log, plutus_data_version_and_payload, DataDecodingError, DecodingResult,
	VersionedDatum, VersionedGenericDatumShape,
};
use cardano_serialization_lib::{BigInt, BigNum, ConstrPlutusData, PlutusData, PlutusList};
use sidechain_domain::{AssetId, AssetName, PolicyId};

#[derive(Debug, Clone)]
pub enum ReserveRedeemer {
	DepositToReserve { governance_version: u64 },
	TransferToIlliquidCirculationSupply,
	UpdateReserve { governance_version: u64 },
	Handover { governance_version: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReserveDatum {
	pub immutable_settings: ReserveImmutableSettings,
	pub mutable_settings: ReserveMutableSettings,
	pub stats: ReserveStats,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReserveImmutableSettings {
	pub token: AssetId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReserveMutableSettings {
	pub total_accrued_function_script_hash: PolicyId,
	pub initial_incentive: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReserveStats {
	pub token_total_amount_transferred: u64,
}

impl From<ReserveRedeemer> for PlutusData {
	fn from(value: ReserveRedeemer) -> Self {
		use ReserveRedeemer::*;
		match value {
			DepositToReserve { governance_version } => {
				PlutusData::new_single_value_constr_plutus_data(
					&BigNum::from(0_u64),
					&PlutusData::new_integer(&BigInt::from(governance_version)),
				)
			},
			TransferToIlliquidCirculationSupply => {
				PlutusData::new_empty_constr_plutus_data(&BigNum::from(1_u64))
			},
			UpdateReserve { governance_version } => {
				PlutusData::new_single_value_constr_plutus_data(
					&BigNum::from(2_u64),
					&PlutusData::new_integer(&BigInt::from(governance_version)),
				)
			},
			Handover { governance_version } => PlutusData::new_single_value_constr_plutus_data(
				&BigNum::from(3_u64),
				&PlutusData::new_integer(&BigInt::from(governance_version)),
			),
		}
	}
}

impl From<ReserveDatum> for PlutusData {
	fn from(value: ReserveDatum) -> Self {
		VersionedGenericDatumShape {
			datum: {
				let mut immutable_settings = PlutusList::new();
				// `t0` field is not used by on-chain code of partner-chains smart-contracts,
				// but only gave a possiblity for user to store "t0" for his own V-function.
				// Not configurable anymore, hardcoded to 0. If users need "t0" for their V-function, they are responsible for storing it somewhere.
				let t0 = PlutusData::new_integer(&BigInt::zero());
				immutable_settings.add(&t0);
				let (policy_id_bytes, asset_name_bytes) = {
					let AssetId { policy_id, asset_name } = value.immutable_settings.token.clone();
					(policy_id.0.to_vec(), asset_name.0.to_vec())
				};
				let token_data: PlutusData = {
					let mut asset_data = PlutusList::new();
					asset_data.add(&PlutusData::new_bytes(policy_id_bytes));
					asset_data.add(&PlutusData::new_bytes(asset_name_bytes));
					PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(
						&BigNum::zero(),
						&asset_data,
					))
				};
				immutable_settings.add(&token_data);

				let mut v_function_hash_and_initial_incentive = PlutusList::new();
				v_function_hash_and_initial_incentive.add(&PlutusData::new_bytes(
					value.mutable_settings.total_accrued_function_script_hash.0.to_vec(),
				));
				v_function_hash_and_initial_incentive.add(&PlutusData::new_integer(&BigInt::from(
					value.mutable_settings.initial_incentive,
				)));

				let mut datum = PlutusList::new();
				datum.add(&PlutusData::new_list(&immutable_settings));
				datum.add(&PlutusData::new_list(&v_function_hash_and_initial_incentive));
				datum.add(&PlutusData::new_integer(
					&value.stats.token_total_amount_transferred.into(),
				));
				PlutusData::new_list(&datum)
			},
			// this empty constructor below is Plutus encoding of `()`
			generic_data: PlutusData::new_empty_constr_plutus_data(&BigNum::zero()),
			version: 0,
		}
		.into()
	}
}

impl TryFrom<PlutusData> for ReserveDatum {
	type Error = DataDecodingError;

	fn try_from(datum: PlutusData) -> DecodingResult<Self> {
		Self::decode(&datum)
	}
}

impl VersionedDatum for ReserveDatum {
	const NAME: &str = "ReserveDatum";

	fn decode(datum: &PlutusData) -> DecodingResult<Self> {
		match plutus_data_version_and_payload(datum) {
			Some((0, datum, _)) => decode_v0_reserve_datum(&datum)
				.ok_or_else(|| decoding_error_and_log(&datum, "ReserveDatum", "invalid data")),
			_ => Err(decoding_error_and_log(datum, "ReserveDatum", "invalid data")),
		}
	}
}

fn decode_v0_reserve_datum(datum: &PlutusData) -> Option<ReserveDatum> {
	let outer_list = datum.as_list()?;
	let mut outer_iter = outer_list.into_iter();

	let immutable_settings_list = outer_iter.next()?.as_list()?;
	let mut immutable_settings_iter = immutable_settings_list.into_iter();
	let _obsolete_t0: u64 = immutable_settings_iter.next()?.as_integer()?.as_u64()?.into();
	let token = decode_token_id_datum(immutable_settings_iter.next()?)?;

	let v_function_hash_and_initial_incentive_list = outer_iter.next()?.as_list()?;
	let mut v_function_hash_and_initial_incentive_iter =
		v_function_hash_and_initial_incentive_list.into_iter();
	let total_accrued_function_script_hash = PolicyId(
		v_function_hash_and_initial_incentive_iter
			.next()?
			.as_bytes()?
			.to_vec()
			.try_into()
			.ok()?,
	);
	let initial_incentive = v_function_hash_and_initial_incentive_iter
		.next()?
		.as_integer()?
		.as_u64()?
		.into();

	let stats = ReserveStats {
		token_total_amount_transferred: outer_iter.next()?.as_integer()?.as_u64()?.into(),
	};

	Some(ReserveDatum {
		immutable_settings: ReserveImmutableSettings { token },
		mutable_settings: ReserveMutableSettings {
			total_accrued_function_script_hash,
			initial_incentive,
		},
		stats,
	})
}

fn decode_token_id_datum(pd: &PlutusData) -> Option<AssetId> {
	let token_id_list = pd
		.as_constr_plutus_data()
		.filter(|constr| constr.alternative() == BigNum::zero())
		.map(|constr| constr.data())?;
	let mut token_id_list_iter = token_id_list.into_iter();
	let policy_id = token_id_list_iter.next()?.as_bytes()?.to_vec();
	let asset_name = token_id_list_iter.next()?.as_bytes()?.to_vec();
	Some(AssetId {
		policy_id: PolicyId(policy_id.try_into().ok()?),
		asset_name: AssetName(asset_name.try_into().ok()?),
	})
}

#[cfg(test)]
mod tests {
	use cardano_serialization_lib::PlutusData;
	use pretty_assertions::assert_eq;
	use sidechain_domain::{AssetName, PolicyId};

	use crate::test_helpers::test_plutus_data;

	use super::{ReserveDatum, ReserveImmutableSettings, ReserveMutableSettings, ReserveStats};

	fn test_datum() -> ReserveDatum {
		ReserveDatum {
			immutable_settings: ReserveImmutableSettings {
				token: sidechain_domain::AssetId {
					policy_id: PolicyId([0; 28]),
					asset_name: AssetName::from_hex_unsafe("aabbcc"),
				},
			},
			mutable_settings: ReserveMutableSettings {
				total_accrued_function_script_hash: PolicyId([2; 28]),
				initial_incentive: 0,
			},
			stats: ReserveStats { token_total_amount_transferred: 1000 },
		}
	}

	fn test_datum_plutus_data() -> PlutusData {
		test_plutus_data!({"list":[
			{"list":[
				{"list":[
					{"int": 0},
					{"constructor":0,
					 "fields":[
						{"bytes": "00000000000000000000000000000000000000000000000000000000"},
						{"bytes": "aabbcc"}]}
				]},
				{"list":[
					{"bytes": "02020202020202020202020202020202020202020202020202020202"},
					{"int": 0}
				]},
				{"int": 1000}
			]},
			{"constructor":0,"fields":[]},
			{"int":0}
		]})
	}

	#[test]
	fn encode() {
		assert_eq!(PlutusData::from(test_datum()), test_datum_plutus_data())
	}

	#[test]
	fn decode() {
		assert_eq!(ReserveDatum::try_from(test_datum_plutus_data()).unwrap(), test_datum())
	}
}
