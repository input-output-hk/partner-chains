use super::{mint_d_param_token_tx, update_d_param_tx};
use crate::d_param::ScriptExUnits;
use crate::{
	csl::{empty_asset_name, TransactionContext},
	test_values::*,
};
use cardano_serialization_lib::{
	Address, ExUnits, Int, LanguageKind, NetworkIdKind, PlutusData, RedeemerTag, ScriptHash,
};
use hex_literal::hex;
use ogmios_client::types::{Asset as OgmiosAsset, OgmiosTx, OgmiosUtxo, OgmiosValue};
use ogmios_client::types::{OgmiosScript, PlutusScript};
use partner_chains_plutus_data::d_param::d_parameter_to_plutus_data;
use sidechain_domain::DParameter;

mod mint_tx {
	use super::*;
	use cardano_serialization_lib::Transaction;

	fn mint_ex_units() -> ExUnits {
		ExUnits::new(&10000u32.into(), &200u32.into())
	}
	fn ex_units() -> ScriptExUnits {
		ScriptExUnits::new().with_mint_ex_units(vec![mint_ex_units(), mint_ex_units()])
	}
	fn mint_d_param_tx() -> Transaction {
		mint_d_param_token_tx(
			&test_validator(),
			&test_policy(),
			&input_d_param(),
			&test_tx_context(),
			ex_units(),
			governance_utxo(),
		)
		.expect("Test transaction should be constructed without error")
	}

	#[test]
	fn both_inputs_are_used_to_cover_transaction() {
		let inputs = mint_d_param_tx().body().inputs();
		// Both inputs are used to cover transaction
		assert_eq!(inputs.get(0).to_string(), lesser_payment_utxo().to_string());
		assert_eq!(inputs.get(1).to_string(), greater_payment_utxo().to_string());
		assert_eq!(inputs.len(), 2)
	}

	#[test]
	fn greater_input_is_selected_as_collateral() {
		let body = mint_d_param_tx().body();
		assert_eq!(
			body.collateral().unwrap().get(0).to_string(),
			greater_payment_utxo().to_string()
		);
	}

	#[test]
	fn redeemer_is_set_correctly() {
		let tx = mint_d_param_tx();

		let redeemers = tx.witness_set().redeemers().unwrap();
		assert_eq!(redeemers.len(), 2);

		let redeemer = redeemers.get(0);
		assert_eq!(redeemer.tag(), RedeemerTag::new_mint());
		assert_eq!(redeemer.index(), 0u64.into());
		assert_eq!(redeemer.data(), PlutusData::new_empty_constr_plutus_data(&0u64.into()));
		assert_eq!(redeemer.ex_units(), mint_ex_units());
	}

	#[test]
	fn collateral_is_set_correctly() {
		let body = mint_d_param_tx().body();

		let collateral_return = body.collateral_return().unwrap();
		assert_eq!(collateral_return.address(), payment_addr());
		let total_collateral = body.total_collateral().unwrap();
		assert_eq!(
			collateral_return.amount().coin().checked_add(&total_collateral).unwrap(),
			greater_payment_utxo().value.lovelace.into()
		);
	}

	#[test]
	fn change_is_returned_correctly() {
		let body = mint_d_param_tx().body();
		let outputs = body.outputs();
		let script_output = (outputs.into_iter())
			.find(|o| o.address() == validator_addr())
			.expect("Should create utxo at validator address");

		let change_output = outputs.into_iter().find(|o| o.address() == payment_addr()).unwrap();
		let coins_sum = change_output
			.amount()
			.coin()
			.checked_add(&script_output.amount().coin())
			.unwrap()
			.checked_add(&body.fee())
			.unwrap();
		assert_eq!(
			coins_sum,
			(greater_payment_utxo().value.lovelace + lesser_payment_utxo().value.lovelace).into()
		);
	}

	#[test]
	fn mints_one_d_param_token_at_validator_address() {
		let body = &mint_d_param_tx().body();

		let d_param_token_mint_amount = body
			.mint()
			.expect("Should mint a token")
			.get(&token_policy_id().into())
			.and_then(|policy| policy.get(0))
			.expect("The minted token should have the D-Param policy")
			.get(&empty_asset_name())
			.expect("The minted token should have an empty asset name");

		assert_eq!(d_param_token_mint_amount, Int::new_i32(1));
	}

	#[test]
	fn should_send_d_param_token_to_validator_address() {
		let body = &mint_d_param_tx().body();

		let outputs = body.outputs();

		let script_output = (outputs.into_iter())
			.find(|o| o.address() == validator_addr())
			.expect("Should create utxo at validator address");

		let output_multi_asset =
			script_output.amount().multiasset().expect("Utxo should contain a native token");

		let d_param_token_amount =
			output_multi_asset.get_asset(&token_policy_id().into(), &empty_asset_name());

		assert_eq!(d_param_token_amount, 1u64.into());
	}

	#[test]
	fn attaches_correct_plutus_data_at_validator_address() {
		let outputs = mint_d_param_tx().body().outputs();

		let script_output = (outputs.into_iter())
			.find(|o| o.address() == validator_addr())
			.expect("Should create utxo at validator address");

		let plutus_data =
			script_output.plutus_data().expect("Utxo should have plutus data attached");

		assert_eq!(plutus_data, expected_plutus_data());
	}
}

mod update_d_parameter {
	use super::*;
	use cardano_serialization_lib::{Redeemer, Transaction};
	use pretty_assertions::assert_eq;

	const SCRIPT_UTXO_LOVELACE: u64 = 1060260;

	fn script_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx { id: [15; 32] },
			index: 1,
			value: OgmiosValue {
				lovelace: SCRIPT_UTXO_LOVELACE,
				native_tokens: vec![(
					token_policy_id(),
					vec![OgmiosAsset { name: vec![], amount: 1 }],
				)]
				.into_iter()
				.collect(),
			},
			address: validator_addr().to_bech32(None).unwrap(),
			..Default::default()
		}
	}

	fn spend_ex_units() -> ExUnits {
		ExUnits::new(&10000u32.into(), &200u32.into())
	}
	fn mint_ex_units() -> ExUnits {
		ExUnits::new(&10000u32.into(), &200u32.into())
	}
	fn ex_units() -> ScriptExUnits {
		ScriptExUnits::new()
			.with_spend_ex_units(vec![spend_ex_units()])
			.with_mint_ex_units(vec![mint_ex_units()])
	}

	fn update_d_param_tx() -> Transaction {
		super::update_d_param_tx(
			&test_validator(),
			&test_policy(),
			&input_d_param(),
			&script_utxo(),
			&test_tx_context(),
			ex_units(),
			governance_utxo(),
		)
		.unwrap()
	}

	#[test]
	fn sets_inputs_correctly() {
		let body = update_d_param_tx().body();
		let inputs = body.inputs();
		// Two utxos are used as payment
		assert_eq!(inputs.get(0).to_string(), lesser_payment_utxo().to_string());
		assert_eq!(inputs.get(1).to_string(), greater_payment_utxo().to_string());
		// Script input goes to inputs
		assert_eq!(inputs.get(2).to_string(), script_utxo().to_string());
		assert_eq!(inputs.len(), 3);

		// The greater payment input goes to collaterals
		assert_eq!(
			body.collateral().unwrap().get(0).to_string(),
			greater_payment_utxo().to_string()
		);
	}

	#[test]
	fn correctly_returns_change() {
		let body = update_d_param_tx().body();
		let outputs = body.outputs();
		// There is a change for payment
		let change_output = outputs.into_iter().find(|o| o.address() == payment_addr()).unwrap();
		// There is 1 d-param token in the validator address output
		let script_output = outputs.into_iter().find(|o| o.address() == validator_addr()).unwrap();
		let coins_sum = change_output
			.amount()
			.coin()
			.checked_add(&script_output.amount().coin())
			.unwrap()
			.checked_add(&body.fee())
			.unwrap();
		assert_eq!(
			coins_sum,
			(greater_payment_utxo().value.lovelace
				+ lesser_payment_utxo().value.lovelace
				+ SCRIPT_UTXO_LOVELACE)
				.into()
		);
		assert_eq!(
			script_output
				.amount()
				.multiasset()
				.unwrap()
				.get_asset(&token_policy_id().into(), &empty_asset_name(),),
			1u64.into()
		);
		assert_eq!(script_output.plutus_data().unwrap(), expected_plutus_data());
	}

	#[test]
	fn spend_redeemer_is_set() {
		let redeemers = update_d_param_tx().witness_set().redeemers().unwrap();

		assert_eq!(redeemers.len(), 2);

		assert_eq!(
			redeemers.get(0),
			Redeemer::new(
				&RedeemerTag::new_spend(),
				&2u64.into(),
				&PlutusData::new_empty_constr_plutus_data(&0u64.into()),
				&spend_ex_units()
			)
		)
	}

	#[test]
	fn collateral_is_returned() {
		let body = update_d_param_tx().body();

		let collateral_return = body.collateral_return().unwrap();
		assert_eq!(collateral_return.address(), payment_addr());
		let total_collateral = body.total_collateral().unwrap();
		assert_eq!(
			collateral_return.amount().coin().checked_add(&total_collateral).unwrap(),
			greater_payment_utxo().value.lovelace.into()
		);
	}

	#[test]
	fn mints_one_governance_policy_token() {
		let body = update_d_param_tx().body();

		let governance_param_mint = body
			.mint()
			.expect("Should mint a token")
			.get(&governance_script_hash())
			.and_then(|policy| policy.get(0))
			.expect("The minted token should have the governance policy policy")
			.get(&empty_asset_name())
			.expect("The minted token should have an empty asset name");

		assert_eq!(governance_param_mint, Int::new_i32(1))
	}
}

fn test_tx_context() -> TransactionContext {
	TransactionContext {
		payment_key: payment_key(),
		payment_key_utxos: vec![
			lesser_payment_utxo(),
			greater_payment_utxo(),
			make_utxo(11u8, 0, 1000000, &payment_addr()),
		],
		network: NetworkIdKind::Testnet,
		protocol_parameters: protocol_parameters(),
	}
}

fn governance_script() -> PlutusScript {
	PlutusScript { language: "PlutusV2".to_string(), cbor: vec![] }
}
fn governance_script_hash() -> ScriptHash {
	crate::plutus_script::PlutusScript::from_cbor(&governance_script().cbor, LanguageKind::PlutusV2)
		.to_csl()
		.hash()
}

fn governance_utxo() -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx { id: [15; 32] },
		index: 0,
		value: OgmiosValue::new_lovelace(1000000),
		address: payment_addr().to_bech32(None).unwrap(),
		script: Some(OgmiosScript::Plutus(governance_script())),
		..Default::default()
	}
}

fn lesser_payment_utxo() -> OgmiosUtxo {
	make_utxo(1u8, 0, 1200000, &payment_addr())
}

fn greater_payment_utxo() -> OgmiosUtxo {
	make_utxo(4u8, 1, 1200001, &payment_addr())
}

fn validator_addr() -> Address {
	Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung").unwrap()
}

fn token_policy_id() -> [u8; 28] {
	hex!("f14241393964259a53ca546af364e7f5688ca5aaa35f1e0da0f951b2")
}

fn input_d_param() -> DParameter {
	DParameter { num_registered_candidates: 30, num_permissioned_candidates: 40 }
}

fn expected_plutus_data() -> PlutusData {
	d_parameter_to_plutus_data(&input_d_param())
}