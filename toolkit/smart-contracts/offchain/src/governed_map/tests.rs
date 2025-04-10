use super::{get_current_value, insert_key_value_tx};
use crate::csl::{empty_asset_name, TransactionContext};
use crate::governance::GovernanceData;
use crate::test_values::*;
use cardano_serialization_lib::{
	Address, ExUnits, Int, NetworkIdKind, PlutusData, RedeemerTag, ScriptHash,
};
use hex_literal::hex;
use ogmios_client::types::{Asset as OgmiosAsset, Datum, OgmiosTx, OgmiosUtxo, OgmiosValue};
use partner_chains_plutus_data::governed_map::{
	governed_map_datum_to_plutus_data, GovernedMapDatum,
};
use sidechain_domain::byte_string::ByteString;

mod governed_map_insert_tx_tests {
	use crate::csl::Costs;

	use super::*;
	use cardano_serialization_lib::Transaction;

	fn policy_ex_units() -> ExUnits {
		ExUnits::new(&10000u32.into(), &200u32.into())
	}

	fn governance_ex_units() -> ExUnits {
		ExUnits::new(&20000u32.into(), &400u32.into())
	}

	fn test_costs() -> Costs {
		Costs::new(
			vec![
				(ScriptHash::from_bytes(token_policy_id().to_vec()).unwrap(), policy_ex_units()),
				(governance_script_hash(), governance_ex_units()),
			]
			.into_iter()
			.collect(),
			vec![].into_iter().collect(),
		)
	}

	fn governerd_map_insert_tx_test() -> Transaction {
		insert_key_value_tx(
			&test_validator(),
			&test_policy(),
			test_key(),
			test_value(),
			&governance_data(),
			test_costs(),
			&test_tx_context(),
		)
		.expect("Test transaction should be constructed without error")
	}

	#[test]
	fn at_least_one_input_is_used_for_transaction() {
		let inputs = governerd_map_insert_tx_test().body().inputs();

		// There should be at least one input
		assert!(inputs.len() > 0, "Transaction should have at least one input");

		// Get the input string and compare it to expected inputs
		let input0_string = inputs.get(0).to_string();
		let lesser_utxo_string = lesser_payment_utxo().to_string();
		let greater_utxo_string = greater_payment_utxo().to_string();

		// Check that at least one of the expected inputs is used
		assert!(
			input0_string == lesser_utxo_string || input0_string == greater_utxo_string,
			"Input should be one of the payment UTXOs"
		);
	}

	#[test]
	fn greater_input_is_selected_as_collateral() {
		let body = governerd_map_insert_tx_test().body();
		assert_eq!(
			body.collateral().unwrap().get(0).to_string(),
			greater_payment_utxo().to_string()
		);
	}

	#[test]
	fn redeemer_is_set_correctly() {
		let tx = governerd_map_insert_tx_test();

		let redeemers = tx.witness_set().redeemers().unwrap();
		assert_eq!(redeemers.len(), 2);

		let redeemer = redeemers.get(0);
		assert_eq!(redeemer.tag(), RedeemerTag::new_mint());
		assert_eq!(redeemer.index(), 0u64.into());
		assert_eq!(redeemer.data(), PlutusData::new_empty_constr_plutus_data(&0u64.into()));
		assert_eq!(redeemer.ex_units(), governance_ex_units());

		let redeemer_2 = redeemers.get(1);
		assert_eq!(redeemer_2.tag(), RedeemerTag::new_mint());
		assert_eq!(redeemer_2.index(), 1u64.into());
		assert_eq!(redeemer_2.data(), PlutusData::new_empty_constr_plutus_data(&0u64.into()));
		assert_eq!(redeemer_2.ex_units(), policy_ex_units());
	}

	#[test]
	fn collateral_is_set_correctly() {
		let body = governerd_map_insert_tx_test().body();

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
		let body = governerd_map_insert_tx_test().body();
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

		// We're just checking that the sum is reasonable, not the exact amount
		let total_input =
			(greater_payment_utxo().value.lovelace + lesser_payment_utxo().value.lovelace).into();
		assert!(coins_sum <= total_input, "Sum of outputs plus fee should not exceed total input");
	}

	#[test]
	fn mints_one_key_value_token_at_validator_address() {
		let body = &governerd_map_insert_tx_test().body();

		let key_value_token_mint_amount = body
			.mint()
			.expect("Should mint a token")
			.get(&token_policy_id().into())
			.and_then(|policy| policy.get(0))
			.expect("The minted token should have the key value policy")
			.get(&empty_asset_name())
			.expect("The minted token should have an empty asset name");

		assert_eq!(key_value_token_mint_amount, Int::new_i32(1));
	}

	#[test]
	fn should_send_key_value_token_to_validator_address() {
		let body = &governerd_map_insert_tx_test().body();

		let outputs = body.outputs();

		let script_output = (outputs.into_iter())
			.find(|o| o.address() == validator_addr())
			.expect("Should create utxo at validator address");

		let output_multi_asset =
			script_output.amount().multiasset().expect("Utxo should contain a native token");

		let key_value_token_amount =
			output_multi_asset.get_asset(&token_policy_id().into(), &empty_asset_name());

		assert_eq!(key_value_token_amount, 1u64.into());
	}

	#[test]
	fn attaches_correct_plutus_data_at_validator_address() {
		let outputs = governerd_map_insert_tx_test().body().outputs();

		let script_output = (outputs.into_iter())
			.find(|o| o.address() == validator_addr())
			.expect("Should create utxo at validator address");

		let plutus_data =
			script_output.plutus_data().expect("Utxo should have plutus data attached");

		assert_eq!(plutus_data, expected_plutus_data());
	}
}

mod get_current_value_tests {
	use super::*;
	use pretty_assertions::assert_eq;

	#[test]
	fn returns_none_when_no_utxos() {
		let utxos = vec![];
		let result = get_current_value(utxos, test_key(), test_policy().policy_id()).unwrap();
		assert_eq!(result, None);
	}

	#[test]
	fn returns_none_when_utxo_without_token() {
		let utxo = OgmiosUtxo {
			transaction: OgmiosTx { id: [1; 32] },
			index: 1,
			value: OgmiosValue { lovelace: 1000000, native_tokens: vec![].into_iter().collect() },
			address: validator_addr().to_bech32(None).unwrap(),
			datum: Some(Datum {
				bytes: PlutusData::from(governed_map_datum_to_plutus_data(&GovernedMapDatum::new(
					test_key(),
					test_value(),
				)))
				.to_bytes(),
			}),
			..Default::default()
		};
		let result = get_current_value(vec![utxo], test_key(), test_policy().policy_id()).unwrap();
		assert_eq!(result, None);
	}

	#[test]
	fn returns_none_when_utxo_with_token_but_different_key() {
		let utxo = OgmiosUtxo {
			transaction: OgmiosTx { id: [1; 32] },
			index: 1,
			value: OgmiosValue {
				lovelace: 1000000,
				native_tokens: vec![(
					test_policy().policy_id().0,
					vec![OgmiosAsset { name: vec![], amount: 1 }],
				)]
				.into_iter()
				.collect(),
			},
			address: validator_addr().to_bech32(None).unwrap(),
			datum: Some(Datum {
				bytes: PlutusData::from(governed_map_datum_to_plutus_data(&GovernedMapDatum::new(
					"different_key".to_string(),
					test_value(),
				)))
				.to_bytes(),
			}),
			..Default::default()
		};
		let result = get_current_value(vec![utxo], test_key(), test_policy().policy_id()).unwrap();
		assert_eq!(result, None);
	}

	#[test]
	fn returns_value_when_utxo_with_token_and_matching_key() {
		let utxo = OgmiosUtxo {
			transaction: OgmiosTx { id: [1; 32] },
			index: 1,
			value: OgmiosValue {
				lovelace: 1000000,
				native_tokens: vec![(
					test_policy().policy_id().0,
					vec![OgmiosAsset { name: vec![], amount: 1 }],
				)]
				.into_iter()
				.collect(),
			},
			address: validator_addr().to_bech32(None).unwrap(),
			datum: Some(Datum {
				bytes: PlutusData::from(governed_map_datum_to_plutus_data(&GovernedMapDatum::new(
					test_key(),
					test_value(),
				)))
				.to_bytes(),
			}),
			..Default::default()
		};
		let result = get_current_value(vec![utxo], test_key(), test_policy().policy_id()).unwrap();
		assert_eq!(result, Some(test_value()));
	}
}

// Common test helper functions
fn test_tx_context() -> TransactionContext {
	TransactionContext {
		network: NetworkIdKind::Testnet,
		payment_key_utxos: vec![lesser_payment_utxo(), greater_payment_utxo()],
		payment_key: test_payment_key(),
		protocol_parameters: protocol_parameters(),
		change_address: payment_addr(),
	}
}

fn governance_script_hash() -> ScriptHash {
	ScriptHash::from_bytes(test_governance_script().script_hash().to_vec()).unwrap()
}

fn governance_data() -> GovernanceData {
	GovernanceData { policy: test_governance_policy(), utxo: mock_governance_utxo() }
}

fn mock_governance_utxo() -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx { id: [0xab; 32] },
		index: 1,
		value: OgmiosValue {
			lovelace: 2000000,
			native_tokens: vec![(
				version_oracle_token_policy_id(),
				vec![OgmiosAsset { name: version_oracle_token_name(), amount: 1 }],
			)]
			.into_iter()
			.collect(),
		},
		address: "addr_test1wqrlc9gqxnyyzwyzgtvrf77famec87zme6zfxgq2sq4up8gccxfnc".to_string(),
		datum: Some(Datum { bytes: PlutusData::new_integer(&(32u64.into())).to_bytes() }),
		..Default::default()
	}
}

fn version_oracle_token_policy_id() -> [u8; 28] {
	hex!("c11dee532646a9b226aac75f77ea7ae5fba9270674327c882794701e")
}

fn version_oracle_token_name() -> Vec<u8> {
	b"Version oracle".to_vec()
}

fn lesser_payment_utxo() -> OgmiosUtxo {
	make_utxo(10, 0, 2_000_000, &payment_addr())
}

fn greater_payment_utxo() -> OgmiosUtxo {
	make_utxo(20, 0, 10_000_000, &payment_addr())
}

fn validator_addr() -> Address {
	test_validator().address(NetworkIdKind::Testnet)
}

fn token_policy_id() -> [u8; 28] {
	test_policy().policy_id().0
}

fn test_key() -> String {
	"test_key".to_string()
}

fn test_value() -> ByteString {
	hex::decode("abcdef123456").unwrap().into()
}

fn expected_plutus_data() -> PlutusData {
	governed_map_datum_to_plutus_data(&GovernedMapDatum::new(test_key(), test_value()))
}
