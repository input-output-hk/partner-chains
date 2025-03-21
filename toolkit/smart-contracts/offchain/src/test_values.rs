use crate::{
	governance::{GovernancePolicyScript, PartnerChainsMultisigPolicy},
	plutus_script::PlutusScript,
};
use cardano_serialization_lib::{Address, Language, PlutusData, PrivateKey};
use hex_literal::hex;
use ogmios_client::{
	query_ledger_state::{
		PlutusCostModels, ProtocolParametersResponse, ReferenceScriptsCosts, ScriptExecutionPrices,
	},
	types::{OgmiosBytesSize, OgmiosTx, OgmiosUtxo, OgmiosValue},
};
use sidechain_domain::StakePoolPublicKey;

pub(crate) fn payment_key() -> PrivateKey {
	PrivateKey::from_normal_bytes(&hex!(
		"cf86dc85e4933424826e846c18d2695689bf65de1fc0c40fcd9389ba1cbdc069"
	))
	.unwrap()
}

pub(crate) const PAYMENT_ADDR: &str =
	"addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy";

pub(crate) fn payment_addr() -> Address {
	Address::from_bech32(PAYMENT_ADDR).unwrap()
}

pub(crate) fn stake_pool_pub_key() -> StakePoolPublicKey {
	StakePoolPublicKey(hex!("bfbee74ab533f40979101057f96de62e95233f2a5216eb16b54106f09fd7350d"))
}

pub(crate) fn test_validator() -> PlutusScript {
	PlutusScript {
		bytes: hex!("4d4c01000022223212001375a009").to_vec(),
		language: Language::new_plutus_v2(),
	}
}

pub(crate) fn test_policy() -> PlutusScript {
	PlutusScript {
		bytes: hex!("49480100002221200101").to_vec(),
		language: Language::new_plutus_v2(),
	}
}

pub(crate) fn test_governance_policy() -> GovernancePolicyScript {
	GovernancePolicyScript::MultiSig(PartnerChainsMultisigPolicy {
		script: test_governance_script(),
		key_hashes: vec![],
		threshold: 0,
	})
}

pub(crate) fn test_governance_script() -> PlutusScript {
	PlutusScript { bytes: hex!("112233").to_vec(), language: Language::new_plutus_v2() }
}

pub(crate) fn test_plutus_data() -> PlutusData {
	PlutusData::new_bytes(vec![1, 2, 3, 4])
}

pub(crate) fn protocol_parameters() -> ProtocolParametersResponse {
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
		max_collateral_inputs: 3,
		collateral_percentage: 150,
		min_fee_reference_scripts: ReferenceScriptsCosts { base: 15.0 },
	}
}

pub(crate) fn make_utxo(id_byte: u8, index: u16, lovelace: u64, addr: &Address) -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx { id: [id_byte; 32] },
		index,
		value: OgmiosValue::new_lovelace(lovelace),
		address: addr.to_bech32(None).unwrap(),
		..Default::default()
	}
}
