#![allow(dead_code)]

use crate::csl::{InputsBuilderExt, TransactionBuilderExt, TransactionContext};
use crate::plutus_script::PlutusScript;
use cardano_serialization_lib::{
	BigNum, DataCost, ExUnits, JsError, MinOutputAdaCalculator, Transaction, TransactionBuilder,
	TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::registered_candidates::block_producer_registration_to_plutus_data;
use sidechain_domain::BlockProducerRegistration;

fn register_tx(
	validator: &PlutusScript,
	block_producer_registration: &BlockProducerRegistration,
	input_utxo: &OgmiosUtxo,
	own_registration_utxos: &[OgmiosUtxo],
	ctx: &TransactionContext,
	validator_redeemer_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	{
		let mut inputs = TxInputsBuilder::new();
		for own_registration_utxo in own_registration_utxos {
			inputs.add_script_utxo_input(
				own_registration_utxo,
				validator,
				validator_redeemer_ex_units.clone(),
			)?;
		}
		inputs.add_key_inputs(&[input_utxo.clone()], &ctx.payment_key_hash())?;
		tx_builder.set_inputs(&inputs);
	}

	{
		let datum = block_producer_registration_to_plutus_data(block_producer_registration);
		let amount_builder = TransactionOutputBuilder::new()
			.with_address(&validator.address(ctx.network))
			.with_plutus_data(&datum)
			.next()?;
		let output = amount_builder.with_coin(&BigNum::zero()).build()?;
		let min_ada = MinOutputAdaCalculator::new(
			&output,
			&DataCost::new_coins_per_byte(
				&ctx.protocol_parameters.min_utxo_deposit_coefficient.into(),
			),
		)
		.calculate_ada()?;
		let output = amount_builder.with_coin(&min_ada).build()?;
		tx_builder.add_output(&output)?;
	}

	tx_builder.balance_update_and_build(ctx)
}

#[cfg(test)]
mod tests {
	use super::register_tx;
	use crate::csl::{OgmiosUtxoExt, TransactionContext};
	use crate::test_values::{self, *};
	use cardano_serialization_lib::{
		Address, BigNum, ExUnits, NetworkIdKind, Transaction, TransactionInputs,
	};
	use ogmios_client::types::OgmiosValue;
	use ogmios_client::types::{OgmiosTx, OgmiosUtxo};
	use partner_chains_plutus_data::registered_candidates::block_producer_registration_to_plutus_data;
	use proptest::{
		array::uniform32,
		collection::{hash_set, vec},
		prelude::*,
	};

	use sidechain_domain::{
		AdaBasedStaking, AuraPublicKey, BlockProducerRegistration, GrandpaPublicKey,
		MainchainAddressHash, MainchainSignature, McTxHash, SidechainPublicKey, SidechainSignature,
		UtxoId, UtxoIndex,
	};

	fn sum_lovelace(utxos: &[OgmiosUtxo]) -> u64 {
		utxos.iter().map(|utxo| utxo.value.lovelace).sum()
	}

	const MIN_UTXO_LOVELACE: u64 = 1000000;
	const FIVE_ADA: u64 = 5000000;

	fn block_producer_registration(consumed_input: UtxoId) -> BlockProducerRegistration {
		BlockProducerRegistration {
			stake_ownership: AdaBasedStaking {
				pub_key: test_values::mainchain_pub_key(),
				signature: MainchainSignature(Vec::new()),
			},
			sidechain_pub_key: SidechainPublicKey(Vec::new()),
			sidechain_signature: SidechainSignature(Vec::new()),
			consumed_input,
			own_pkh: MainchainAddressHash([0; 28]),
			aura_pub_key: AuraPublicKey(Vec::new()),
			grandpa_pub_key: GrandpaPublicKey(Vec::new()),
		}
	}

	fn lesser_payment_utxo() -> OgmiosUtxo {
		make_utxo(1u8, 0, 1200000, &payment_addr())
	}

	fn greater_payment_utxo() -> OgmiosUtxo {
		make_utxo(4u8, 1, 1200001, &payment_addr())
	}

	fn input_utxo() -> OgmiosUtxo {
		make_utxo(11u8, 0, 1000000, &payment_addr())
	}

	fn validator_addr() -> Address {
		Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung")
			.unwrap()
	}

	#[test]
	fn register_tx_regression_test() {
		let ex_units = ExUnits::new(&0u32.into(), &0u32.into());
		let payment_key_utxos = vec![lesser_payment_utxo(), greater_payment_utxo(), input_utxo()];
		let ctx = TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: payment_key_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		};
		let own_registration_utxos = vec![payment_key_utxos.get(1).unwrap().clone()];
		let input_utxo = payment_key_utxos.first().unwrap();
		let block_producer_registration = block_producer_registration(input_utxo.to_domain());
		let tx = register_tx(
			&test_values::test_validator(),
			&block_producer_registration,
			input_utxo,
			&own_registration_utxos,
			&ctx,
			ex_units,
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
		let outputs = body.outputs();

		let script_output = outputs.into_iter().find(|o| o.address() == validator_addr()).unwrap();
		let coins_sum = script_output.amount().coin().checked_add(&body.fee()).unwrap();
		assert_eq!(
			coins_sum,
			(greater_payment_utxo().value.lovelace + lesser_payment_utxo().value.lovelace).into()
		);
		assert_eq!(
			script_output.plutus_data().unwrap(),
			block_producer_registration_to_plutus_data(&block_producer_registration)
		);
	}

	fn register_transaction_balancing_test(payment_utxos: Vec<OgmiosUtxo>) {
		let payment_key_utxos = payment_utxos.clone();
		let ctx = TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: payment_key_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		};
		let input_utxo = payment_key_utxos.first().unwrap();
		let block_producer_registration = block_producer_registration(input_utxo.to_domain());
		let own_registration_utxos = if payment_utxos.len() >= 2 {
			vec![payment_utxos.get(1).unwrap().clone()]
		} else {
			Vec::new()
		};
		let tx = register_tx(
			&test_values::test_validator(),
			&block_producer_registration,
			input_utxo,
			&own_registration_utxos,
			&ctx,
			ExUnits::new(&BigNum::zero(), &BigNum::zero()),
		)
		.unwrap();

		let validator_address = &test_values::test_validator().address(ctx.network);

		used_inputs_lovelace_equals_outputs_and_fee(&tx, &payment_key_utxos.clone());
		fee_is_less_than_one_and_half_ada(&tx);
		output_at_validator_has_register_candidate_datum(
			&tx,
			&block_producer_registration,
			validator_address,
		);
		spends_own_registration_utxos(&tx, &own_registration_utxos);
	}

	fn match_inputs(inputs: &TransactionInputs, payment_utxos: &[OgmiosUtxo]) -> Vec<OgmiosUtxo> {
		inputs
			.into_iter()
			.map(|input| {
				payment_utxos
					.iter()
					.find(|utxo| utxo.to_csl_tx_input() == *input)
					.unwrap()
					.clone()
			})
			.collect()
	}

	fn used_inputs_lovelace_equals_outputs_and_fee(tx: &Transaction, payment_utxos: &[OgmiosUtxo]) {
		let used_inputs: Vec<OgmiosUtxo> = match_inputs(&tx.body().inputs(), payment_utxos);
		let used_inputs_value: u64 = sum_lovelace(&used_inputs);
		let outputs_lovelace_sum: u64 = tx
			.body()
			.outputs()
			.into_iter()
			.map(|output| {
				let value: u64 = output.amount().coin().into();
				value
			})
			.sum();
		let fee: u64 = tx.body().fee().into();
		// Used inputs are qual to the sum of the outputs plus the fee
		assert_eq!(used_inputs_value, outputs_lovelace_sum + fee);
	}

	// Exact fee depends on inputs and outputs, but it definately is less than 1.5 ADA
	fn fee_is_less_than_one_and_half_ada(tx: &Transaction) {
		assert!(tx.body().fee() <= 1500000u64.into());
	}

	fn output_at_validator_has_register_candidate_datum(
		tx: &Transaction,
		block_producer_registration: &BlockProducerRegistration,
		validator_address: &Address,
	) {
		let outputs = tx.body().outputs();
		let validator_output =
			outputs.into_iter().find(|o| o.address() == *validator_address).unwrap();
		assert_eq!(
			validator_output.plutus_data().unwrap(),
			block_producer_registration_to_plutus_data(block_producer_registration)
		);
	}

	fn spends_own_registration_utxos(tx: &Transaction, own_registration_utxos: &[OgmiosUtxo]) {
		let inputs = tx.body().inputs();
		assert!(own_registration_utxos
			.iter()
			.all(|p| inputs.into_iter().any(|i| *i == p.to_csl_tx_input())));
	}

	proptest! {
		#[test]
		fn spends_input_utxo_and_outputs_to_validator_address(payment_utxos in arb_payment_utxos(10)
			.prop_filter("Inputs total lovelace too low", |utxos| sum_lovelace(utxos) > 4000000)) {
			register_transaction_balancing_test(payment_utxos)
		}
	}

	prop_compose! {
		// Set is needed to be used, because we have to avoid UTXOs with the same id.
		fn arb_payment_utxos(n: usize)
			(utxo_ids in hash_set(arb_utxo_id(), 1..n))
			(utxo_ids in Just(utxo_ids.clone()), values in vec(arb_utxo_lovelace(), utxo_ids.len())
		) -> Vec<OgmiosUtxo> {
			utxo_ids.into_iter().zip(values.into_iter()).map(|(utxo_id, value)| OgmiosUtxo {
				transaction: OgmiosTx { id: utxo_id.tx_hash.0 },
				index: utxo_id.index.0,
				value,
				address: payment_addr().to_bech32(None).unwrap(),
				..Default::default()
			}).collect()
		}
	}

	prop_compose! {
		fn arb_utxo_lovelace()(value in MIN_UTXO_LOVELACE..FIVE_ADA) -> OgmiosValue {
			OgmiosValue::new_lovelace(value)
		}
	}

	prop_compose! {
		fn arb_utxo_id()(tx_hash in uniform32(0u8..255u8), index in any::<u16>()) -> UtxoId {
			UtxoId {
				tx_hash: McTxHash(tx_hash),
				index: UtxoIndex(index),
			}
		}
	}
}
