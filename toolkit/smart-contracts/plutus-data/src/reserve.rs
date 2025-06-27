//! Plutus data types for reserve functionality.
use crate::{
	DataDecodingError, DecodingResult, VersionedDatum, VersionedGenericDatum,
	decoding_error_and_log, plutus_data_version_and_payload,
};
use cardano_serialization_lib::{BigInt, BigNum, ConstrPlutusData, PlutusData, PlutusList};
use sidechain_domain::{AssetId, AssetName, PolicyId};

#[derive(Debug, Clone)]
/// Redeemer for reserve
pub enum ReserveRedeemer {
	/// Deposit tokens to the reserve
	DepositToReserve = 0,
	/// Releases funds from the reserve to the illiquid supply
	ReleaseFromReserve = 1,
	/// Update mutable reserve management system settings
	UpdateReserve = 2,
	/// Releases all the remaining funds from the reserve to the illiquid supply
	Handover = 3,
}

#[derive(Debug, Clone, PartialEq)]
/// Datum of the reserve storing settings and stats
pub struct ReserveDatum {
	/// Reserve settings that *are not* changeable after creation
	pub immutable_settings: ReserveImmutableSettings,
	/// Reserve settings that *are* changeable after creation
	pub mutable_settings: ReserveMutableSettings,
	/// Reserve stats
	pub stats: ReserveStats,
}

#[derive(Debug, Clone, PartialEq)]
/// Type representing reserve settings that *are not* changeable after creation
pub struct ReserveImmutableSettings {
	/// Unused, set to 0
	pub t0: u64,
	/// Reserve token asset Id
	pub token: AssetId,
}

#[derive(Debug, Clone, PartialEq)]
/// Type representing reserve settings that *are* changeable after creation
pub struct ReserveMutableSettings {
	/// Asset name of the 'total accrued function' minting policy, also called V-function,
	/// that determines the maximum amount of tokens that can be released from the reserve at any given moment.
	/// This is accomplished by minting v-function tokens with the policy, and comparing the released amount
	/// to the token amount in the reserver validator script.
	pub total_accrued_function_asset_name: PolicyId,
	/// Initial amount of tokens to deposit. They must be present in the payment wallet.
	pub initial_incentive: u64,
}

#[derive(Debug, Clone, PartialEq)]
/// Type representing continuously changing information about the reserve
pub struct ReserveStats {
	/// Total amount of tokens released from reserve
	pub token_total_amount_transferred: u64,
}

impl From<ReserveRedeemer> for PlutusData {
	fn from(value: ReserveRedeemer) -> Self {
		PlutusData::new_empty_constr_plutus_data(&BigNum::from(value as u64))
	}
}

impl From<ReserveDatum> for PlutusData {
	fn from(value: ReserveDatum) -> Self {
		VersionedGenericDatum {
			datum: {
				let mut immutable_settings = PlutusList::new();
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
					value.mutable_settings.total_accrued_function_asset_name.0.to_vec(),
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
			appendix: PlutusData::new_empty_constr_plutus_data(&BigNum::zero()),
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
			Some(VersionedGenericDatum { version: 0, datum, .. }) => {
				decode_v0_reserve_datum(&datum)
					.ok_or_else(|| decoding_error_and_log(&datum, "ReserveDatum", "invalid data"))
			},
			_ => Err(decoding_error_and_log(datum, "ReserveDatum", "unversioned datum")),
		}
	}
}

impl ReserveDatum {
	/// Calculates new reserve after withdrawal by adding `amount` to `token_total_amount_transferred`.
	pub fn after_withdrawal(self, amount: u64) -> Self {
		Self {
			stats: ReserveStats {
				token_total_amount_transferred: self.stats.token_total_amount_transferred + amount,
			},
			..self
		}
	}
}

fn decode_v0_reserve_datum(datum: &PlutusData) -> Option<ReserveDatum> {
	let outer_list = datum.as_list()?;
	let mut outer_iter = outer_list.into_iter();

	let immutable_settings_list = outer_iter.next()?.as_list()?;
	let mut immutable_settings_iter = immutable_settings_list.into_iter();
	let t0: u64 = immutable_settings_iter.next()?.as_integer()?.as_u64()?.into();
	let token = decode_token_id_datum(immutable_settings_iter.next()?)?;

	let mutable_settings_list = outer_iter.next()?.as_list()?;
	let mut mutable_settings_iter = mutable_settings_list.into_iter();
	let total_accrued_function_script_hash =
		PolicyId(mutable_settings_iter.next()?.as_bytes()?.to_vec().try_into().ok()?);
	let initial_incentive = mutable_settings_iter.next()?.as_integer()?.as_u64()?.into();

	let stats = ReserveStats {
		token_total_amount_transferred: outer_iter.next()?.as_integer()?.as_u64()?.into(),
	};

	Some(ReserveDatum {
		immutable_settings: ReserveImmutableSettings { t0, token },
		mutable_settings: ReserveMutableSettings {
			total_accrued_function_asset_name: total_accrued_function_script_hash,
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

#[derive(Debug, Clone)]
/// Redeemer for illiquid circulation supply
pub enum IlliquidCirculationSupplyRedeemer {
	/// Deposit tokens to the supply
	DepositMoreToSupply = 0,
	/// Withdraw from the illiquid supply
	WithdrawFromSupply = 1,
}

impl From<IlliquidCirculationSupplyRedeemer> for PlutusData {
	fn from(value: IlliquidCirculationSupplyRedeemer) -> Self {
		PlutusData::new_integer(&BigInt::from(value as u64))
	}
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
				t0: 0,
				token: sidechain_domain::AssetId {
					policy_id: PolicyId([0; 28]),
					asset_name: AssetName::from_hex_unsafe("aabbcc"),
				},
			},
			mutable_settings: ReserveMutableSettings {
				total_accrued_function_asset_name: PolicyId([2; 28]),
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
