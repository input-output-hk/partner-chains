#![allow(dead_code)]

use crate::csl::{
	add_collateral_inputs, add_mint_script_token, add_output_with_one_script_token,
	convert_cost_models, get_builder_config, key_hash_address, ogmios_utxos_to_csl,
};
use cardano_serialization_lib::{
	ChangeConfig, CoinSelectionStrategyCIP2, Ed25519KeyHash, ExUnits, JsError, NetworkIdKind,
	PlutusData, PlutusScript, Transaction, TransactionBuilder,
};
use ogmios_client::{query_ledger_state::ProtocolParametersResponse, types::OgmiosUtxo};
use partner_chains_plutus_data::d_param::DParamDatum;
use sidechain_domain::DParameter;

#[allow(clippy::too_many_arguments)]
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
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(protocol_parameters)?);
	// The essence of transaction: mint token and set output with it
	add_mint_script_token(&mut tx_builder, validator, mint_witness_ex_units)?;
	add_output_with_one_script_token(
		&mut tx_builder,
		validator,
		&d_parameter_to_plutus_data(d_parameter),
		network,
		protocol_parameters.min_utxo_deposit_coefficient,
	)?;
	// Set things required for transaction to succeed
	add_collateral_inputs(&mut tx_builder, collaterals, payment_key_hash)?;
	tx_builder
		.calc_script_data_hash(&convert_cost_models(&protocol_parameters.plutus_cost_models))?;
	tx_builder.add_required_signer(payment_key_hash);
	tx_builder.add_inputs_from_and_change_with_collateral_return(
		&ogmios_utxos_to_csl(payment_utxos)?,
		CoinSelectionStrategyCIP2::LargestFirstMultiAsset,
		&ChangeConfig::new(&key_hash_address(payment_key_hash, network)),
		&protocol_parameters.collateral_percentage.into(),
	)?;
	tx_builder.build_tx()
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
		let payment_addr =
			Address::from_bech32("addr_test1vqp4a7r0zc3pw2qkhw0fz6h2s6grktydxtrj3t2unw2890sfgt0kq")
				.unwrap();
		let validator_addr =
			Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung")
				.unwrap();
		let collateral_value = 7000000;
		let collateral = make_utxo(7u8, 0, collateral_value, &payment_addr);
		let payment_utxo = make_utxo(4u8, 1, 4000000, &payment_addr);
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
		assert_eq!(collateral_return.address(), payment_addr);
		let total_collateral = body.total_collateral().unwrap();
		assert_eq!(
			collateral_return.amount().coin().checked_add(&total_collateral).unwrap(),
			collateral_value.into()
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
