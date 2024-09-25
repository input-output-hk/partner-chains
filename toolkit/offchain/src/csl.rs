#![allow(dead_code)]

use cardano_serialization_lib::{
	Address, AssetName, Assets, BigNum, CostModel, Costmdls, Credential, EnterpriseAddress,
	ExUnitPrices, ExUnits, JsError, Language, LanguageKind, LinearFee, MultiAsset, NetworkIdKind,
	ScriptHash, TransactionBuilderConfigBuilder, UnitInterval, Value,
};
use ogmios_client::{
	query_ledger_state::{PlutusCostModels, ProtocolParametersResponse},
	transactions::OgmiosBudget,
	types::OgmiosValue,
};

pub(crate) fn plutus_script_address(
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

pub(crate) fn convert_cost_models(m: &PlutusCostModels) -> Costmdls {
	let mut mdls = Costmdls::new();
	mdls.insert(&Language::new_plutus_v1(), &CostModel::from(m.plutus_v1.to_owned()));
	mdls.insert(&Language::new_plutus_v2(), &CostModel::from(m.plutus_v2.to_owned()));
	mdls.insert(&Language::new_plutus_v3(), &CostModel::from(m.plutus_v3.to_owned()));
	mdls
}

pub(crate) fn get_builder_config(
	protocol_parameters: &ProtocolParametersResponse,
) -> Result<cardano_serialization_lib::TransactionBuilderConfig, JsError> {
	let sep = &protocol_parameters.script_execution_prices;

	TransactionBuilderConfigBuilder::new()
		.fee_algo(&linear_fee(protocol_parameters))
		.pool_deposit(&convert_value(&protocol_parameters.stake_pool_deposit)?.coin())
		.key_deposit(&convert_value(&protocol_parameters.stake_credential_deposit)?.coin())
		.max_value_size(protocol_parameters.max_value_size.bytes)
		.max_tx_size(protocol_parameters.max_transaction_size.bytes)
		.ex_unit_prices(&ExUnitPrices::new(
			&UnitInterval::new(&(*sep.memory.numer()).into(), &(*sep.memory.denom()).into()),
			&UnitInterval::new(&(*sep.cpu.numer()).into(), &(*sep.cpu.denom()).into()),
		))
		.coins_per_utxo_byte(&protocol_parameters.min_utxo_deposit_coefficient.into())
		.build()
}

pub(crate) fn linear_fee(protocol_parameters: &ProtocolParametersResponse) -> LinearFee {
	let constant: BigNum = protocol_parameters.min_fee_constant.lovelace.into();
	LinearFee::new(&protocol_parameters.min_fee_coefficient.into(), &constant)
}

pub(crate) fn convert_value(value: &OgmiosValue) -> Result<Value, JsError> {
	let mut multiasset = MultiAsset::new();
	value.native_tokens.iter().try_for_each(|(policy_id, assets)| {
		let mut csl_assets = Assets::new();
		assets.iter().try_for_each(|asset| {
			let amount: u64 =
				asset.amount.try_into().map_err(|_| JsError::from_str("Amount too large"))?;
			csl_assets.insert(&AssetName::new(asset.name.clone())?, &amount.into());
			Ok::<(), JsError>(())
		})?;
		multiasset.insert(&ScriptHash::from(*policy_id), &csl_assets);
		Ok::<(), JsError>(())
	})?;
	if multiasset.len() == 0 {
		Ok(Value::new(&value.lovelace.into()))
	} else {
		Ok(Value::new_with_assets(&value.lovelace.into(), &multiasset))
	}
}

pub(crate) fn convert_ex_units(v: &OgmiosBudget) -> ExUnits {
	ExUnits::new(&v.memory.into(), &v.cpu.into())
}

#[cfg(test)]
mod tests {}
