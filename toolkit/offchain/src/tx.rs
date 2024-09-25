/// This file contains the functions to create and sign a transaction.
/// It is implemented with cardano-serialization-lib, not pallas.
use crate::ogmios::{OgmiosBudget, OgmiosUtxo, OgmiosValue, ProtocolParametersResponse};
use cardano_serialization_lib::{
	Address, BigNum, CostModel, Costmdls, ExUnitPrices, ExUnits, JsError, Language, LinearFee,
	PlutusData, PlutusScript, PlutusWitness, PrivateKey, Redeemer, RedeemerTag, Transaction,
	TransactionBuilder, TransactionBuilderConfigBuilder, TransactionHash, TransactionInput,
	TxInputsBuilder, UnitInterval, Value, Vkey, Vkeywitness, Vkeywitnesses,
};
use pallas_addresses::ShelleyAddress;

pub fn make_deregister_tx(
	collateral: &OgmiosUtxo,
	utxos: &Vec<OgmiosUtxo>,
	addr: &ShelleyAddress,
	own_pkh: [u8; 28],
	protocol_parameters: ProtocolParametersResponse,
	validator_bytes: Vec<u8>,
	validator_redeemer_ex_units: ExUnits,
) -> Result<Transaction, JsError> {
	let addr = to_csl_address(addr);
	let config = get_builder_config(&protocol_parameters)?;

	let mut tx_builder = TransactionBuilder::new(&config);

	let mut tx_inputs_builder = TxInputsBuilder::new();
	utxos.into_iter().for_each(|utxo| {
		let amount: BigNum = convert_value(&utxo.value).coin();
		let hash: [u8; 32] = hex::decode(utxo.transaction.id.clone()).unwrap().try_into().unwrap();
		tx_inputs_builder.add_key_input(
			&From::from(own_pkh),
			&TransactionInput::new(&TransactionHash::from(hash), utxo.index.into()),
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
			&TransactionInput::new(&TransactionHash::from(hash), utxo.index.into()),
			&Value::new(&amount),
		);
	});
	tx_builder.set_inputs(&tx_inputs_builder);
	let collateral_builder = get_collateral_builder(collateral, own_pkh);

	tx_builder.set_collateral(&collateral_builder);
	tx_builder
		.calc_script_data_hash(&convert_cost_models(&protocol_parameters.plutus_cost_models))?;
	tx_builder.add_required_signer(&From::from(own_pkh));
	// This should to be the last step before building the transaction
	tx_builder.add_change_if_needed(&addr)?;

	let tx: Transaction = tx_builder.build_tx()?;
	println!("Fee {:?}", tx.body().fee());
	Ok(tx)
}

// For deregister it is easy, because registration Utxo to spent has enough ada, so this collateral seems to be "pro-forma".
fn get_collateral_builder(collateral: &OgmiosUtxo, own_pkh: [u8; 28]) -> TxInputsBuilder {
	let mut collateral_builder = TxInputsBuilder::new();

	let amount: BigNum = convert_value(&collateral.value).coin();
	let hash: [u8; 32] =
		hex::decode(collateral.transaction.id.clone()).unwrap().try_into().unwrap();
	collateral_builder.add_key_input(
		&From::from(own_pkh),
		&TransactionInput::new(&TransactionHash::from(hash), collateral.index.into()),
		&Value::new(&amount),
	);
	collateral_builder
}

fn get_builder_config(
	protocol_parameters: &ProtocolParametersResponse,
) -> Result<cardano_serialization_lib::TransactionBuilderConfig, JsError> {
	let builder = TransactionBuilderConfigBuilder::new();
	let builder = builder.fee_algo(&linear_fee(protocol_parameters));
	let builder =
		builder.pool_deposit(&convert_value(&protocol_parameters.stake_pool_deposit).coin());
	let builder =
		builder.key_deposit(&convert_value(&protocol_parameters.stake_credential_deposit).coin());
	let builder = builder.max_value_size(protocol_parameters.clone().max_value_size.bytes);
	let builder =
		builder.max_tx_size(protocol_parameters.clone().max_transaction_size.bytes.clone());
	// TODO: present in the protocol parameters, but as string in format "n/d"
	let builder = builder.ex_unit_prices(&ExUnitPrices::new(
		&UnitInterval::new(&577u32.into(), &10000u32.into()),
		&UnitInterval::new(&721u32.into(), &10000000u32.into()),
	));
	// TODO: perhaps 'minUtxoDepositCoefficient'
	let builder = builder.coins_per_utxo_byte(&4310u32.into());
	builder.build()
}

pub fn sign_tx(tx: &Transaction, prv_key: &[u8]) -> Transaction {
	let tx_hash: [u8; 32] = sidechain_domain::crypto::blake2b(tx.body().to_bytes().as_ref());
	let pk = PrivateKey::from_normal_bytes(&prv_key).unwrap();
	let sig = pk.sign(&tx_hash);
	let mut witness_set = tx.witness_set();
	let mut vkeywitnesses = witness_set.vkeys().unwrap_or_else(|| Vkeywitnesses::new());
	vkeywitnesses.add(&Vkeywitness::new(&Vkey::new(&pk.to_public()), &sig));
	witness_set.set_vkeys(&vkeywitnesses);
	Transaction::new(&tx.body(), &witness_set, tx.auxiliary_data())
}

fn to_csl_address(addr: &ShelleyAddress) -> Address {
	Address::from_bech32(&addr.to_bech32().unwrap()).unwrap()
}

fn linear_fee(protocol_parameters: &ProtocolParametersResponse) -> LinearFee {
	let constant: BigNum = match protocol_parameters.min_fee_constant {
		OgmiosValue::Ada { lovelace } => lovelace.into(),
	};
	LinearFee::new(&protocol_parameters.min_fee_coefficient.into(), &constant)
}

// TODO: native tokens conversion is missing
fn convert_value(value: &OgmiosValue) -> Value {
	match value {
		OgmiosValue::Ada { lovelace } => Value::new(&(lovelace.clone().into())),
	}
}

fn convert_cost_models(v: &serde_json::Value) -> Costmdls {
	fn extract(key: &str, map: &serde_json::Map<String, serde_json::Value>) -> CostModel {
		let v: Vec<i128> = map
			.get(key)
			.unwrap()
			.as_array()
			.unwrap()
			.iter()
			.map(|item| item.as_i64().unwrap().into())
			.collect();
		CostModel::from(v)
	}

	let mut mdls = Costmdls::new();
	match v {
		serde_json::Value::Object(m) => {
			mdls.insert(&Language::new_plutus_v1(), &extract("plutus:v1", m));
			mdls.insert(&Language::new_plutus_v2(), &extract("plutus:v2", m));
			mdls.insert(&Language::new_plutus_v3(), &extract("plutus:v3", m));
		},
		_ => panic!("todo; invalid cost models"),
	}
	mdls
}

pub fn convert_ex_units(v: &OgmiosBudget) -> ExUnits {
	ExUnits::new(&v.memory.into(), &v.cpu.into())
}

/// This can be used to read a transaction from bytes and print it.
/// Used for exploration of CTL-made transactions.
#[allow(dead_code)]
pub fn print_tx_from_bytes(bytes: &[u8]) {
	let tx = Transaction::from_bytes(bytes.to_vec()).unwrap();
	println!("{:#?}", tx);
}
