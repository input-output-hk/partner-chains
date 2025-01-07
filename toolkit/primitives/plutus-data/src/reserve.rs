use crate::VersionedGenericDatumShape;
use cardano_serialization_lib::{BigInt, BigNum, ConstrPlutusData, PlutusData, PlutusList};
use sidechain_domain::{PolicyId, TokenId};

pub struct ReserveDatum {
	pub immutable_settings: ReserveImmutableSettings,
	pub mutable_settings: ReserveMutableSettings,
	pub stats: ReserveStats,
}

pub struct ReserveImmutableSettings {
	pub t0: u64,
	pub token: TokenId,
}

pub struct ReserveMutableSettings {
	pub total_accrued_function_script_hash: PolicyId,
	pub initial_incentive: u64,
}

pub struct ReserveStats {
	pub token_total_amount_transferred: u64,
}

impl From<ReserveDatum> for PlutusData {
	fn from(value: ReserveDatum) -> Self {
		VersionedGenericDatumShape {
			datum: {
				let mut immutable_settings = PlutusList::new();
				immutable_settings
					.add(&PlutusData::new_integer(&BigInt::from(value.immutable_settings.t0)));
				let (policy_id_bytes, asset_name_bytes) =
					match value.immutable_settings.token.clone() {
						TokenId::Ada => (vec![], vec![]),
						TokenId::AssetId { policy_id, asset_name } => {
							(policy_id.0.to_vec(), asset_name.0.to_vec())
						},
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
