#![allow(dead_code)]

use crate::csl::{get_builder_config, InputsBuilderExt, TransactionBuilderExt, TransactionContext};
use crate::plutus_script::PlutusScript;
use cardano_serialization_lib::{
	ExUnits, JsError, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::d_param::d_parameter_to_plutus_data;
use sidechain_domain::DParameter;

#[allow(clippy::too_many_arguments)]
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

	tx_builder.set_required_fields_and_build(ctx)
}

#[allow(clippy::too_many_arguments)]
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
	inputs.add_key_inputs(&ctx.payment_utxos, &ctx.payment_key_hash)?;
	tx_builder.set_inputs(&inputs);

	tx_builder.add_output_with_one_script_token(
		validator,
		&d_parameter_to_plutus_data(d_parameter),
		ctx,
	)?;

	tx_builder.set_required_fields_and_build(ctx)
}

#[cfg(test)]
mod tests {
	use super::{mint_d_param_token_tx, update_d_param_tx};
	use crate::{
		csl::{empty_asset_name, TransactionContext},
		plutus_script::PlutusScript,
	};
	use cardano_serialization_lib::{
		Address, Ed25519KeyHash, ExUnits, Int, LanguageKind, NetworkIdKind, PlutusData,
		RedeemerTag, ScriptHash,
	};
	use hex_literal::hex;
	use ogmios_client::{
		query_ledger_state::{PlutusCostModels, ProtocolParametersResponse, ScriptExecutionPrices},
		types::{Asset as OgmiosAsset, OgmiosBytesSize, OgmiosTx, OgmiosUtxo, OgmiosValue},
	};
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
		// Payment inputs are script utxo and payment utxo
		assert_eq!(
			inputs.get(0).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
		// Collateral input goes to collaterals
		assert_eq!(
			body.collateral().unwrap().get(0).to_string(),
			"0707070707070707070707070707070707070707070707070707070707070707#0"
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
		assert_eq!(coins_sum, payment_utxo().value.lovelace.into());
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
			collateral_utxo().value.lovelace.into()
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
		// Script input goes to inputs
		assert_eq!(
			inputs.get(0).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
		// Payment input goes to inputs
		assert_eq!(
			inputs.get(1).to_string(),
			"0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f#0"
		);
		// Collateral input goes to collaterals
		assert_eq!(
			body.collateral().unwrap().get(0).to_string(),
			"0707070707070707070707070707070707070707070707070707070707070707#0"
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
		assert_eq!(coins_sum, (payment_utxo().value.lovelace + script_utxo_lovelace).into());
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
			collateral_utxo().value.lovelace.into()
		);
	}

	fn test_tx_context() -> TransactionContext {
		TransactionContext {
			payment_key_hash: payment_key_hash(),
			collaterals: vec![collateral_utxo()],
			payment_utxos: vec![payment_utxo()],
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		}
	}

	fn test_script() -> PlutusScript {
		PlutusScript {
			bytes: hex!("4d4c01000022223212001375a009").to_vec(),
			language: LanguageKind::PlutusV2,
		}
	}

	fn payment_key_hash() -> Ed25519KeyHash {
		hex!("035ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be").into()
	}

	fn collateral_utxo() -> OgmiosUtxo {
		make_utxo(7u8, 0, 7000000, &payment_addr())
	}

	fn payment_utxo() -> OgmiosUtxo {
		make_utxo(4u8, 1, 4000000, &payment_addr())
	}

	fn validator_addr() -> Address {
		Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung")
			.unwrap()
	}

	fn payment_addr() -> Address {
		Address::from_bech32("addr_test1vqp4a7r0zc3pw2qkhw0fz6h2s6grktydxtrj3t2unw2890sfgt0kq")
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

	fn protocol_parameters() -> ProtocolParametersResponse {
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
		}
	}

	fn make_utxo(id_byte: u8, index: u16, lovelace: u64, addr: &Address) -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx { id: [id_byte; 32] },
			index,
			value: OgmiosValue::new_lovelace(lovelace),
			address: addr.to_bech32(None).unwrap(),
			..Default::default()
		}
	}
}
