#![allow(dead_code)]

use cardano_serialization_lib::{
	Address, AssetName, Assets, BigNum, CostModel, Costmdls, Credential, EnterpriseAddress,
	ExUnitPrices, ExUnits, JsError, Language, LanguageKind, LinearFee, MultiAsset, NetworkIdKind,
	ScriptHash, TransactionBuilderConfig, TransactionBuilderConfigBuilder, UnitInterval, Value,
};
use ogmios_client::{
	query_ledger_state::{PlutusCostModels, ProtocolParametersResponse},
	transactions::OgmiosBudget,
	types::OgmiosValue,
};

/// Builds an CSL `Address` for plutus script from the data obtained from smart contracts.
pub fn plutus_script_address(
	script_bytes: &[u8],
	network: NetworkIdKind,
	language: LanguageKind,
) -> Address {
	// Before hashing the script, we need to prepend with byte 0x02, because this is PlutusV2 script
	let mut buf: Vec<u8> = vec![language_kind_to_u8(language)];
	buf.extend(script_bytes);
	let script_hash = sidechain_domain::crypto::blake2b(buf.as_slice());
	EnterpriseAddress::new(
		network_id_kind_to_u8(network),
		&Credential::from_scripthash(&script_hash.into()),
	)
	.to_address()
}

pub fn payment_address(key_bytes: &[u8], network: NetworkIdKind) -> Address {
	let key_hash = sidechain_domain::crypto::blake2b(key_bytes);
	EnterpriseAddress::new(
		network_id_kind_to_u8(network),
		&Credential::from_keyhash(&key_hash.into()),
	)
	.to_address()
}

fn network_id_kind_to_u8(network: NetworkIdKind) -> u8 {
	match network {
		NetworkIdKind::Mainnet => 1,
		NetworkIdKind::Testnet => 0,
	}
}

fn language_kind_to_u8(language: LanguageKind) -> u8 {
	match language {
		LanguageKind::PlutusV1 => 1,
		LanguageKind::PlutusV2 => 2,
		LanguageKind::PlutusV3 => 3,
	}
}

/// Creates a CSL [`TransactionBuilderConfig`] for given [`ProtocolParametersResponse`].
/// This function is not unit-testable because [`TransactionBuilderConfig`] has no public getters.
pub(crate) fn get_builder_config(
	protocol_parameters: &ProtocolParametersResponse,
) -> Result<TransactionBuilderConfig, JsError> {
	TransactionBuilderConfigBuilder::new()
		.fee_algo(&linear_fee(protocol_parameters))
		.pool_deposit(&convert_value(&protocol_parameters.stake_pool_deposit)?.coin())
		.key_deposit(&convert_value(&protocol_parameters.stake_credential_deposit)?.coin())
		.max_value_size(protocol_parameters.max_value_size.bytes)
		.max_tx_size(protocol_parameters.max_transaction_size.bytes)
		.ex_unit_prices(&ExUnitPrices::new(
			&ratio_to_unit_interval(&protocol_parameters.script_execution_prices.memory),
			&ratio_to_unit_interval(&protocol_parameters.script_execution_prices.cpu),
		))
		.coins_per_utxo_byte(&protocol_parameters.min_utxo_deposit_coefficient.into())
		.build()
}

fn linear_fee(protocol_parameters: &ProtocolParametersResponse) -> LinearFee {
	let constant: BigNum = protocol_parameters.min_fee_constant.lovelace.into();
	LinearFee::new(&protocol_parameters.min_fee_coefficient.into(), &constant)
}

fn ratio_to_unit_interval(ratio: &fraction::Ratio<u64>) -> UnitInterval {
	UnitInterval::new(&(*ratio.numer()).into(), &(*ratio.denom()).into())
}

/// Coverts ogmios value to CSL value.
/// It could fail if the input contains negative values, for example ogmios values representing burn.
pub(crate) fn convert_value(value: &OgmiosValue) -> Result<Value, JsError> {
	if value.native_tokens.len() > 0 {
		let mut multiasset = MultiAsset::new();
		for (policy_id, assets) in value.native_tokens.iter() {
			let mut csl_assets = Assets::new();
			for asset in assets.iter() {
				let amount: u64 =
					asset.amount.try_into().map_err(|_| JsError::from_str("Amount too large"))?;
				csl_assets.insert(&AssetName::new(asset.name.clone())?, &amount.into());
			}
			multiasset.insert(&ScriptHash::from(*policy_id), &csl_assets);
		}
		Ok(Value::new_with_assets(&value.lovelace.into(), &multiasset))
	} else {
		Ok(Value::new(&value.lovelace.into()))
	}
}

/// Conversion of ogmios-client cost models to CSL
pub(crate) fn convert_cost_models(m: &PlutusCostModels) -> Costmdls {
	let mut mdls = Costmdls::new();
	mdls.insert(&Language::new_plutus_v1(), &CostModel::from(m.plutus_v1.to_owned()));
	mdls.insert(&Language::new_plutus_v2(), &CostModel::from(m.plutus_v2.to_owned()));
	mdls.insert(&Language::new_plutus_v3(), &CostModel::from(m.plutus_v3.to_owned()));
	mdls
}

/// Conversion of ogmios-client budget to CSL execution units
pub(crate) fn convert_ex_units(v: &OgmiosBudget) -> ExUnits {
	ExUnits::new(&v.memory.into(), &v.cpu.into())
}

#[cfg(test)]
mod tests {
	use crate::csl::plutus_script_address;
	use cardano_serialization_lib::{AssetName, Language, LanguageKind, NetworkIdKind};
	use hex_literal::hex;
	use ogmios_client::{
		query_ledger_state::{PlutusCostModels, ProtocolParametersResponse, ScriptExecutionPrices},
		transactions::OgmiosBudget,
		types::{Asset, OgmiosBytesSize, OgmiosValue},
	};

	use super::payment_address;

	#[test]
	fn candidates_script_address_test() {
		let address = plutus_script_address(
			&crate::untyped_plutus::tests::CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS,
			NetworkIdKind::Testnet,
			LanguageKind::PlutusV2,
		);
		assert_eq!(
			address.to_bech32(None).unwrap(),
			"addr_test1wq7vcwawqa29a5a2z7q8qs6k0cuvp6z2puvd8xx7vasuajq86paxz"
		);
	}

	#[test]
	fn payment_address_test() {
		let address = payment_address(
			&hex!("a35ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be7e3c5e88"),
			NetworkIdKind::Testnet,
		);
		assert_eq!(
			address.to_bech32(None).unwrap(),
			"addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy"
		)
	}

	#[test]
	fn linear_fee_test() {
		let fee = super::linear_fee(&test_protocol_parameters());
		assert_eq!(fee.constant(), 155381u32.into());
		assert_eq!(fee.coefficient(), 44u32.into());
	}

	#[test]
	fn ratio_to_unit_interval_test() {
		let ratio = fraction::Ratio::new(577, 10000);
		let unit_interval = super::ratio_to_unit_interval(&ratio);
		assert_eq!(unit_interval.numerator(), 577u64.into());
		assert_eq!(unit_interval.denominator(), 10000u64.into());
	}

	#[test]
	fn convert_value_without_multi_asset_test() {
		let ogmios_value = OgmiosValue::new_lovelace(1234567);
		let value = super::convert_value(&ogmios_value).unwrap();
		assert_eq!(value.coin(), 1234567u64.into());
		assert_eq!(value.multiasset(), None);
	}

	#[test]
	fn convert_value_with_multi_asset_test() {
		let ogmios_value = OgmiosValue {
			lovelace: 1234567,
			native_tokens: vec![
				([0u8; 28], vec![Asset { name: vec![], amount: 111 }]),
				(
					[1u8; 28],
					vec![
						Asset { name: hex!("222222").to_vec(), amount: 222 },
						Asset { name: hex!("333333").to_vec(), amount: 333 },
					],
				),
			]
			.into_iter()
			.collect(),
		};
		let value = super::convert_value(&ogmios_value).unwrap();
		assert_eq!(value.coin(), 1234567u64.into());
		let multiasset = value.multiasset().unwrap();
		assert_eq!(
			multiasset.get_asset(&[0u8; 28].into(), &AssetName::new(vec![]).unwrap()),
			111u64.into()
		);
		assert_eq!(
			multiasset
				.get_asset(&[1u8; 28].into(), &AssetName::new(hex!("222222").to_vec()).unwrap()),
			222u64.into()
		);
		assert_eq!(
			multiasset
				.get_asset(&[1u8; 28].into(), &AssetName::new(hex!("333333").to_vec()).unwrap()),
			333u64.into()
		);
	}

	#[test]
	fn convert_cost_models_test() {
		let cost_models =
			super::convert_cost_models(&test_protocol_parameters().plutus_cost_models);
		assert_eq!(cost_models.keys().len(), 3);
		assert_eq!(
			cost_models
				.get(&Language::new_plutus_v1())
				.unwrap()
				.get(0)
				.unwrap()
				.as_i32_or_nothing()
				.unwrap(),
			898148
		);
		assert_eq!(
			cost_models
				.get(&Language::new_plutus_v2())
				.unwrap()
				.get(1)
				.unwrap()
				.as_i32_or_nothing()
				.unwrap(),
			10
		);
		assert_eq!(
			cost_models
				.get(&Language::new_plutus_v3())
				.unwrap()
				.get(0)
				.unwrap()
				.as_i32_or_nothing()
				.unwrap(),
			-900
		);
	}

	#[test]
	fn convert_ex_values_test() {
		let ex_units = super::convert_ex_units(&OgmiosBudget { memory: 1000, cpu: 2000 });
		assert_eq!(ex_units.mem(), 1000u64.into());
		assert_eq!(ex_units.steps(), 2000u64.into());
	}

	fn test_protocol_parameters() -> ProtocolParametersResponse {
		ProtocolParametersResponse {
			min_fee_coefficient: 44,
			min_fee_constant: OgmiosValue::new_lovelace(155381),
			stake_pool_deposit: OgmiosValue::new_lovelace(500000000),
			stake_credential_deposit: OgmiosValue::new_lovelace(2000000),
			max_value_size: OgmiosBytesSize { bytes: 5000 },
			max_transaction_size: OgmiosBytesSize { bytes: 16384 },
			min_utxo_deposit_coefficient: 4310,
			script_execution_prices: ScriptExecutionPrices {
				memory: fraction::Ratio::new_raw(577, 10000),
				cpu: fraction::Ratio::new_raw(721, 10000000),
			},
			plutus_cost_models: PlutusCostModels {
				plutus_v1: vec![898148, 53384111, 14333],
				plutus_v2: vec![43053543, 10],
				plutus_v3: vec![-900, 166917843],
			},
		}
	}
}
