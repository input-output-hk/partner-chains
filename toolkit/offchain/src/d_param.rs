#![allow(dead_code)]

use crate::csl::{get_builder_config, InputsBuilderExt, TransactionBuilderExt, TransactionContext};
use crate::plutus_script::PlutusScript;
use cardano_serialization_lib::{
	ExUnits, JsError, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::d_param::d_parameter_to_plutus_data;
use sidechain_domain::DParameter;

fn mint_d_param_token_tx(
	validator: &PlutusScript,
	d_parameter: &DParameter,
	ctx: &TransactionContext,
	mint_witness_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// The essence of transaction: mint token and set output with it
	tx_builder.add_mint_one_script_token(validator, mint_witness_ex_units)?;
	tx_builder.add_output_with_one_script_token(
		validator,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	tx_builder.balance_update_and_build(ctx)
}

fn update_d_param_tx(
	validator: &PlutusScript,
	d_parameter: &DParameter,
	script_utxo: &OgmiosUtxo,
	ctx: &TransactionContext,
	validator_redeemer_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	let mut inputs = TxInputsBuilder::new();
	inputs.add_script_utxo_input(script_utxo, validator, validator_redeemer_ex_units)?;
	tx_builder.set_inputs(&inputs);

	tx_builder.add_output_with_one_script_token(
		validator,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	tx_builder.balance_update_and_build(ctx)
}

#[cfg(test)]
mod tests {
	use super::{mint_d_param_token_tx, update_d_param_tx};
	use crate::{
		csl::{empty_asset_name, TransactionContext},
		test_values::*,
	};
	use cardano_serialization_lib::{
		Address, ExUnits, Int, NetworkIdKind, PlutusData, RedeemerTag, ScriptHash,
	};
	use hex_literal::hex;
	use ogmios_client::types::{Asset as OgmiosAsset, OgmiosTx, OgmiosUtxo, OgmiosValue};
	use partner_chains_plutus_data::d_param::d_parameter_to_plutus_data;
	use sidechain_domain::DParameter;

	#[test]
	fn mint_d_param_token_tx_regression_test() {
		// We know the expected values were obtained with the correct code
		let ex_units = ExUnits::new(&10000u32.into(), &200u32.into());

		let tx = mint_d_param_token_tx(
			&test_script(),
			&input_d_param(),
			&test_tx_context(),
			ex_units.clone(),
		)
		.unwrap();

		let body = tx.body();
		let inputs = body.inputs();
		// Both inputs are used to cover transaction
		assert_eq!(
			inputs.get(0).to_string(),
			"0101010101010101010101010101010101010101010101010101010101010101#0"
		);
		assert_eq!(
			inputs.get(1).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
		// The greater input is selected as collateral
		assert_eq!(
			body.collateral().unwrap().get(0).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
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
			(greater_payment_utxo().value.lovelace + lesser_payment_utxo().value.lovelace).into()
		);
		let token_policy_id =
			ScriptHash::from(hex!("6fdad2bafb138ef29280dc1bacb7d468fdc7bc3e93966a6edf0022a0"));
		assert_eq!(
			script_output
				.amount()
				.multiasset()
				.unwrap()
				.get_asset(&token_policy_id, &empty_asset_name(),),
			1u64.into()
		);
		assert_eq!(script_output.plutus_data().unwrap(), expected_plutus_data());
		// This token is minted in the transaction
		let mint = body.mint().unwrap();
		assert_eq!(
			mint.get(&token_policy_id)
				.unwrap()
				.get(0)
				.unwrap()
				.get(&empty_asset_name())
				.unwrap(),
			Int::new_i32(1)
		);
		// Redeemer is set
		let redeemers = tx.witness_set().redeemers().unwrap();
		assert_eq!(redeemers.len(), 1);
		let redeemer = redeemers.get(0);
		assert_eq!(redeemer.tag(), RedeemerTag::new_mint());
		assert_eq!(redeemer.index(), 0u64.into());
		assert_eq!(redeemer.data(), PlutusData::new_empty_constr_plutus_data(&0u64.into()));
		assert_eq!(redeemer.ex_units(), ex_units);
		// script_data_hash check
		assert_eq!(
			tx.body().script_data_hash().unwrap().to_hex(),
			"5b95e874a40a87b017ee7827a7dccf7331d2b647190eddcde7f0edaba4393662"
		);
		// Collateral return must be set
		let collateral_return = body.collateral_return().unwrap();
		assert_eq!(collateral_return.address(), payment_addr());
		let total_collateral = body.total_collateral().unwrap();
		assert_eq!(
			collateral_return.amount().coin().checked_add(&total_collateral).unwrap(),
			greater_payment_utxo().value.lovelace.into()
		);
	}

	#[test]
	fn update_d_param_tx_regression_test() {
		// We know the expected values were obtained with the correct code
		let script_utxo_lovelace = 1060260;
		let script_utxo = OgmiosUtxo {
			transaction: OgmiosTx { id: [15; 32] },
			index: 0,
			value: OgmiosValue {
				lovelace: script_utxo_lovelace,
				native_tokens: vec![(
					token_policy_id(),
					vec![OgmiosAsset { name: vec![], amount: 1 }],
				)]
				.into_iter()
				.collect(),
			},
			address: validator_addr().to_bech32(None).unwrap(),
			..Default::default()
		};

		let ex_units = ExUnits::new(&10000u32.into(), &200u32.into());

		let tx = update_d_param_tx(
			&test_script(),
			&input_d_param(),
			&script_utxo,
			&test_tx_context(),
			ex_units.clone(),
		)
		.unwrap();

		let body = tx.body();
		let inputs = body.inputs();
		// The greater payment input goes to inputs, the lesser one is not used, because script_utxo already covers some part of outputs
		assert_eq!(
			inputs.get(0).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
		// Script input goes to inputs
		assert_eq!(
			inputs.get(1).to_string(),
			"0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f#0"
		);
		// The greater payment input goes to collaterals
		assert_eq!(
			body.collateral().unwrap().get(0).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
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
			(greater_payment_utxo().value.lovelace + script_utxo_lovelace).into()
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
		// No token is minted in the transaction
		assert!(body.mint().is_none());
		// Spend redeemer is set
		let redeemers = tx.witness_set().redeemers().unwrap();
		assert_eq!(redeemers.len(), 1);
		let redeemer = redeemers.get(0);
		assert_eq!(redeemer.tag(), RedeemerTag::new_spend());
		// Index is 1 because the script input is the 2nd input, if it was the 3rd input it would be 2, etc.
		assert_eq!(redeemer.index(), 1u64.into());
		assert_eq!(redeemer.data(), PlutusData::new_empty_constr_plutus_data(&0u64.into()));
		assert_eq!(redeemer.ex_units(), ex_units);
		// script_data_hash check
		assert_eq!(
			tx.body().script_data_hash().unwrap().to_hex(),
			"1b80a34a767a594124163993ee6206fdfa83fc5cb22b267e70e9173fc24b663f"
		);
		// Collateral return must be set
		let collateral_return = body.collateral_return().unwrap();
		assert_eq!(collateral_return.address(), payment_addr());
		let total_collateral = body.total_collateral().unwrap();
		assert_eq!(
			collateral_return.amount().coin().checked_add(&total_collateral).unwrap(),
			greater_payment_utxo().value.lovelace.into()
		);
	}

	fn test_tx_context() -> TransactionContext {
		TransactionContext {
			payment_key: payment_key(),
			payment_utxos: vec![
				lesser_payment_utxo(),
				greater_payment_utxo(),
				make_utxo(11u8, 0, 100000, &payment_addr()),
			],
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		}
	}

	fn lesser_payment_utxo() -> OgmiosUtxo {
		make_utxo(1u8, 0, 1200000, &payment_addr())
	}

	fn greater_payment_utxo() -> OgmiosUtxo {
		make_utxo(4u8, 1, 1200001, &payment_addr())
	}

	fn validator_addr() -> Address {
		Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung")
			.unwrap()
	}

	fn token_policy_id() -> [u8; 28] {
		hex!("6fdad2bafb138ef29280dc1bacb7d468fdc7bc3e93966a6edf0022a0")
	}

	fn input_d_param() -> DParameter {
		DParameter { num_registered_candidates: 30, num_permissioned_candidates: 40 }
	}

	fn expected_plutus_data() -> PlutusData {
		d_parameter_to_plutus_data(&input_d_param())
	}
}
