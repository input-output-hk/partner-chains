#![allow(dead_code)]

use crate::csl::{get_builder_config, InputsBuilderExt, TransactionBuilderExt, TransactionContext};
use crate::plutus_script::PlutusScript;
use cardano_serialization_lib::{
	ExUnits, JsError, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::permissioned_candidates::permissioned_candidates_to_plutus_data;
use sidechain_domain::PermissionedCandidateData;

#[allow(clippy::too_many_arguments)]
fn mint_permissioned_candidates_token_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	permissioned_candidates: &[PermissionedCandidateData],
	ctx: &TransactionContext,
	mint_witness_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// The essence of transaction: mint token and set output with it
	tx_builder.add_mint_one_script_token(policy, mint_witness_ex_units)?;
	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&permissioned_candidates_to_plutus_data(permissioned_candidates),
		ctx,
	)?;

	tx_builder.balance_update_and_build(ctx)
}

#[allow(clippy::too_many_arguments)]
fn update_permissioned_candidates_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	permissioned_candidates: &[PermissionedCandidateData],
	script_utxo: &OgmiosUtxo,
	ctx: &TransactionContext,
	validator_redeemer_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	let mut inputs = TxInputsBuilder::new();
	inputs.add_script_utxo_input(script_utxo, policy, validator_redeemer_ex_units)?;
	tx_builder.set_inputs(&inputs);

	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&permissioned_candidates_to_plutus_data(permissioned_candidates),
		ctx,
	)?;

	tx_builder.balance_update_and_build(ctx)
}

#[cfg(test)]
mod tests {
	use super::{mint_permissioned_candidates_token_tx, update_permissioned_candidates_tx};
	use crate::{
		csl::{empty_asset_name, TransactionContext},
		test_values::*,
	};
	use cardano_serialization_lib::{
		Address, ExUnits, Int, NetworkIdKind, PlutusData, RedeemerTag,
	};
	use hex_literal::hex;
	use ogmios_client::types::{Asset as OgmiosAsset, OgmiosTx, OgmiosUtxo, OgmiosValue};
	use partner_chains_plutus_data::permissioned_candidates::permissioned_candidates_to_plutus_data;
	use sidechain_domain::{
		AuraPublicKey, GrandpaPublicKey, PermissionedCandidateData, SidechainPublicKey,
	};

	#[test]
	fn mint_permissioned_candiates_token_tx_regression_test() {
		// We know the expected values were obtained with the correct code
		let ex_units = ExUnits::new(&10000u32.into(), &200u32.into());

		let tx = mint_permissioned_candidates_token_tx(
			&test_validator(),
			&test_policy(),
			&permissioned_candidates(),
			&test_tx_context(),
			ex_units.clone(),
		)
		.unwrap();

		let body = tx.body();
		let inputs = body.inputs();
		// Both payment utxos are used as inputs
		assert_eq!(
			inputs.get(0).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
		assert_eq!(
			inputs.get(1).to_string(),
			"0707070707070707070707070707070707070707070707070707070707070707#0"
		);
		// The greater payment utxo is used as collateral
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
		assert_eq!(
			script_output
				.amount()
				.multiasset()
				.unwrap()
				.get_asset(&token_policy_id().into(), &empty_asset_name(),),
			1u64.into()
		);
		assert_eq!(script_output.plutus_data().unwrap(), expected_plutus_data());
		// This token is minted in the transaction
		let mint = body.mint().unwrap();
		assert_eq!(
			mint.get(&token_policy_id().into())
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
	fn update_permissioned_candidates_tx_regression_test() {
		// We know the expected values were obtained with the correct code
		let script_utxo_lovelace = 1952430;
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

		let tx = update_permissioned_candidates_tx(
			&test_validator(),
			&test_policy(),
			&permissioned_candidates(),
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
		// The greater payment utxo is used as collateral
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
			payment_key_utxos: vec![
				lesser_payment_utxo(),
				greater_payment_utxo(),
				make_utxo(14u8, 0, 400000, &payment_addr()),
			],
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		}
	}

	fn lesser_payment_utxo() -> OgmiosUtxo {
		make_utxo(7u8, 0, 1700000, &payment_addr())
	}

	fn greater_payment_utxo() -> OgmiosUtxo {
		make_utxo(4u8, 1, 1800000, &payment_addr())
	}

	fn validator_addr() -> Address {
		Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung")
			.unwrap()
	}

	fn token_policy_id() -> [u8; 28] {
		hex!("f14241393964259a53ca546af364e7f5688ca5aaa35f1e0da0f951b2")
	}

	fn permissioned_candidates() -> Vec<PermissionedCandidateData> {
		vec![
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(
					hex!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
						.into(),
				),
				aura_public_key: AuraPublicKey(
					hex!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").into(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc").into(),
				),
			},
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(
					hex!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")
						.into(),
				),
				aura_public_key: AuraPublicKey(
					hex!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee").into(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").into(),
				),
			},
		]
	}

	fn expected_plutus_data() -> PlutusData {
		permissioned_candidates_to_plutus_data(&permissioned_candidates())
	}
}
