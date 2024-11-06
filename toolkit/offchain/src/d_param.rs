#![allow(dead_code)]

use crate::csl::{
	convert_cost_models, empty_asset_name, key_hash_address, ogmios_utxo_to_tx_input,
	plutus_script_address, simple_collateral_builder,
};
use cardano_serialization_lib::{
	Assets, BigNum, DataCost, Ed25519KeyHash, ExUnits, Int, JsError, LanguageKind,
	MinOutputAdaCalculator, MintBuilder, MintWitness, MultiAsset, NetworkIdKind, PlutusData,
	PlutusScript, PlutusScriptSource, Redeemer, RedeemerTag, Transaction, TransactionBuilder,
	TransactionOutputBuilder, TxInputsBuilder, Value,
};
use ogmios_client::{query_ledger_state::ProtocolParametersResponse, types::OgmiosUtxo};
use partner_chains_plutus_data::d_param::DParamDatum;
use sidechain_domain::DParameter;

fn mint_token_tx(
	validator: &PlutusScript,
	d_parameter: &DParameter,
	payment_key_hash: &Ed25519KeyHash,
	collaterals: &[OgmiosUtxo],
	payment_utxos: &[OgmiosUtxo],
	network: NetworkIdKind,
	protocol_parameters: &ProtocolParametersResponse,
	mint_witness_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let config = crate::csl::get_builder_config(protocol_parameters)?;

	let mut tx_builder = TransactionBuilder::new(&config);

	let mut tx_inputs_builder = TxInputsBuilder::new();
	for utxo in payment_utxos.iter() {
		let amount: BigNum = crate::csl::convert_value(&utxo.value)?.coin();
		let input = ogmios_utxo_to_tx_input(utxo);
		tx_inputs_builder.add_key_input(payment_key_hash, &input, &Value::new(&amount));
	}
	tx_builder.set_inputs(&tx_inputs_builder);

	let collateral_builder = simple_collateral_builder(collaterals, payment_key_hash);
	tx_builder.set_collateral(&collateral_builder?);

	let mut mint_builder = MintBuilder::new();
	let validator_source = PlutusScriptSource::new(&validator);
	let mint_witness = MintWitness::new_plutus_script(
		&validator_source,
		&Redeemer::new(
			&RedeemerTag::new_mint(),
			&0u32.into(),
			&PlutusData::new_empty_constr_plutus_data(&0u32.into()),
			&mint_witness_ex_units,
		),
	);
	mint_builder.add_asset(&mint_witness, &empty_asset_name(), &Int::new_i32(1))?;
	tx_builder.set_mint_builder(&mint_builder);

	let output = output_with_d_param_datum(
		d_parameter,
		network,
		validator,
		protocol_parameters.min_utxo_deposit_coefficient,
	)?;
	tx_builder.add_output(&output)?;

	tx_builder
		.calc_script_data_hash(&convert_cost_models(&protocol_parameters.plutus_cost_models))?;
	tx_builder.add_required_signer(payment_key_hash);
	let change_address = key_hash_address(payment_key_hash, network);
	tx_builder.add_change_if_needed(&change_address)?;
	tx_builder.build_tx()
}

// This creates output on the validator address with datum that has 1 token and keep d-param in datum.
fn output_with_d_param_datum(
	d_parameter: &DParameter,
	network: NetworkIdKind,
	validator: &PlutusScript,
	min_utxo_deposit_coefficient: u64,
) -> Result<cardano_serialization_lib::TransactionOutput, JsError> {
	let datum = d_parameter_to_plutus_data(d_parameter);
	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&plutus_script_address(&validator.bytes(), network, LanguageKind::PlutusV2))
		.with_plutus_data(&datum)
		.next()?;
	let mut ma = MultiAsset::new();
	let mut assets = Assets::new();
	assets.insert(&empty_asset_name(), &1u64.into());
	ma.insert(&validator.hash(), &assets);
	let output = amount_builder.with_coin_and_asset(&0u64.into(), &ma).build()?;
	let min_ada = MinOutputAdaCalculator::new(
		&output,
		&DataCost::new_coins_per_byte(&min_utxo_deposit_coefficient.into()),
	)
	.calculate_ada()?;
	let output = amount_builder.with_coin_and_asset(&min_ada, &ma).build()?;
	Ok(output)
}

fn d_parameter_to_plutus_data(d_parameter: &DParameter) -> PlutusData {
	let d_parameter: DParamDatum = d_parameter.clone().into();
	d_parameter.into()
}

#[cfg(test)]
mod tests {
	use super::mint_token_tx;
	use crate::csl::empty_asset_name;
	use cardano_serialization_lib::{
		Address, ExUnits, Int, NetworkIdKind, PlutusData, PlutusList, PlutusScript, RedeemerTag,
		ScriptHash,
	};
	use hex_literal::hex;
	use ogmios_client::{
		query_ledger_state::{PlutusCostModels, ProtocolParametersResponse, ScriptExecutionPrices},
		types::{OgmiosBytesSize, OgmiosTx, OgmiosUtxo, OgmiosValue},
	};
	use sidechain_domain::DParameter;

	#[test]
	fn mint_token_regression_test() {
		// We know the expected values were obtained with the correct code
		let collateral = make_utxo(7u8, 0, 72345678);
		let payment_utxo = make_utxo(4u8, 1, 4000000);
		let payment_addr =
			Address::from_bech32("addr_test1vqp4a7r0zc3pw2qkhw0fz6h2s6grktydxtrj3t2unw2890sfgt0kq")
				.unwrap();
		let validator_addr =
			Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung")
				.unwrap();
		let pub_key_hash = hex!("035ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be").into();
		let ex_units = ExUnits::new(&10000u32.into(), &200u32.into());

		let tx = mint_token_tx(
			&PlutusScript::new_v2(hex!("4d4c01000022223212001375a009").to_vec()),
			&DParameter { num_registered_candidates: 30, num_permissioned_candidates: 40 },
			&pub_key_hash,
			&[collateral],
			&[payment_utxo],
			NetworkIdKind::Testnet,
			&protocol_parameters(),
			ex_units.clone(),
		)
		.unwrap();

		let body = tx.body();
		let inputs = body.inputs();
		// Payment input goes to inputs
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
		assert!(outputs.into_iter().find(|o| o.address() == payment_addr).is_some());
		// There is 1 d-param token in the validator address output
		let script_output = outputs.into_iter().find(|o| o.address() == validator_addr).unwrap();
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
		let expected_plutus_data = {
			let mut list = PlutusList::new();
			list.add(&PlutusData::new_integer(&40.into()));
			list.add(&PlutusData::new_integer(&30.into()));
			PlutusData::new_list(&list)
		};
		assert_eq!(script_output.plutus_data().unwrap(), expected_plutus_data);
		// This token is minted in the transaction
		let mint = tx.body().mint().unwrap();
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
		}
	}

	fn make_utxo(id_byte: u8, index: u16, lovelace: u64) -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx { id: [id_byte; 32] },
			index,
			value: OgmiosValue::new_lovelace(lovelace),
			..Default::default()
		}
	}
}
