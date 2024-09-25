#![allow(dead_code)]

/// This file contains the functions to create and sign a transaction.
/// It is implemented with cardano-serialization-lib, not pallas.
//use crate::ogmios::{OgmiosBudget, OgmiosUtxo, OgmiosValue, ProtocolParametersResponse};
use cardano_serialization_lib::{
	Address, BigNum, ExUnits, JsError, PlutusData, PlutusScript, PlutusWitness, PrivateKey,
	Redeemer, RedeemerTag, Transaction, TransactionBuilder, TransactionHash, TransactionInput,
	TxInputsBuilder, Value, Vkey, Vkeywitness, Vkeywitnesses,
};
use ogmios_client::{query_ledger_state::ProtocolParametersResponse, types::OgmiosUtxo};

pub fn make_deregister_tx(
	collateral: &OgmiosUtxo,
	utxos: &[OgmiosUtxo],
	addr: &Address,
	own_pkh: [u8; 28],
	protocol_parameters: &ProtocolParametersResponse,
	validator_bytes: Vec<u8>,
	validator_redeemer_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let config = crate::csl::get_builder_config(protocol_parameters)?;

	let mut tx_builder = TransactionBuilder::new(&config);

	let mut tx_inputs_builder = TxInputsBuilder::new();
	utxos.iter().try_for_each(|utxo| {
		let amount: BigNum = crate::csl::convert_value(&utxo.value)?.coin();
		let hash: [u8; 32] = hex::decode(utxo.transaction.id).unwrap().try_into().unwrap();
		tx_inputs_builder.add_key_input(
			&From::from(own_pkh),
			&TransactionInput::new(&TransactionHash::from(hash), utxo.index),
			&Value::new(&amount),
		);
		// This redeemer is required for the following plutus script input, it is a dummy in this case.
		// Important parts are: index - to match the script, in this it is easy, because there is only one script
		//                      ex_units - they are zero before sending the transaction for evaluation, in the subsequent call it should set with evaluation result
		let redeemer = Redeemer::new(
			&RedeemerTag::new_spend(),
			&0u32.into(),
			&PlutusData::new_empty_constr_plutus_data(&0u32.into()),
			&validator_redeemer_ex_units,
		);
		// This input is not present in the build transacition body but as a witness (redeemer above as well).
		// When interacting with UTXO created by some plutus script, the full bytes of input script are required.
		tx_inputs_builder.add_plutus_script_input(
			&PlutusWitness::new_without_datum(
				&PlutusScript::new_v2(validator_bytes.clone()),
				&redeemer,
			),
			&TransactionInput::new(&TransactionHash::from(hash), utxo.index),
			&Value::new(&amount),
		);
		Ok::<(), JsError>(())
	})?;
	tx_builder.set_inputs(&tx_inputs_builder);
	let collateral_builder = get_collateral_builder(collateral, own_pkh);

	tx_builder.set_collateral(&collateral_builder?);
	tx_builder.calc_script_data_hash(&crate::csl::convert_cost_models(
		&protocol_parameters.plutus_cost_models,
	))?;
	tx_builder.add_required_signer(&From::from(own_pkh));
	// This should to be the last step before building the transaction
	tx_builder.add_change_if_needed(&addr)?;

	let tx: Transaction = tx_builder.build_tx()?;
	println!("Fee {:?}", tx.body().fee());
	Ok(tx)
}

// For deregister it is easy, because registration Utxo to spent has enough ada, so this collateral seems to be "pro-forma".
fn get_collateral_builder(
	collateral: &OgmiosUtxo,
	own_pkh: [u8; 28],
) -> Result<TxInputsBuilder, JsError> {
	let mut collateral_builder = TxInputsBuilder::new();

	let amount: BigNum = crate::csl::convert_value(&collateral.value)?.coin();
	let hash: [u8; 32] = collateral.transaction.id;
	collateral_builder.add_key_input(
		&From::from(own_pkh),
		&TransactionInput::new(&TransactionHash::from(hash), collateral.index),
		&Value::new(&amount),
	);
	Ok(collateral_builder)
}

pub fn sign_tx(tx: &Transaction, prv_key: &[u8]) -> Transaction {
	let tx_hash: [u8; 32] = sidechain_domain::crypto::blake2b(tx.body().to_bytes().as_ref());
	let pk = PrivateKey::from_normal_bytes(prv_key).unwrap();
	let sig = pk.sign(&tx_hash);
	let mut witness_set = tx.witness_set();
	let mut vkeywitnesses = witness_set.vkeys().unwrap_or_else(Vkeywitnesses::new);
	vkeywitnesses.add(&Vkeywitness::new(&Vkey::new(&pk.to_public()), &sig));
	witness_set.set_vkeys(&vkeywitnesses);
	Transaction::new(&tx.body(), &witness_set, tx.auxiliary_data())
}

/// This can be used to read a transaction from bytes and print it.
/// Used for exploration of CTL-made transactions.
#[allow(dead_code)]
pub fn print_tx_from_bytes(bytes: &[u8]) {
	let tx = Transaction::from_bytes(bytes.to_vec()).unwrap();
	println!("{:#?}", tx);
}
