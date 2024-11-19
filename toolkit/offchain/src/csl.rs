#![allow(dead_code)]

use crate::{plutus_script::PlutusScript, untyped_plutus::datum_to_uplc_plutus_data};
use cardano_serialization_lib::*;
use ogmios_client::{
	query_ledger_state::{PlutusCostModels, ProtocolParametersResponse},
	transactions::OgmiosBudget,
	types::{OgmiosUtxo, OgmiosValue},
};

pub(crate) fn plutus_script_hash(script_bytes: &[u8], language: LanguageKind) -> [u8; 28] {
	// Before hashing the script, we need to prepend with byte denoting the language.
	let mut buf: Vec<u8> = vec![language_kind_to_u8(language)];
	buf.extend(script_bytes);
	sidechain_domain::crypto::blake2b(buf.as_slice())
}

/// Builds an CSL `Address` for plutus script from the data obtained from smart contracts.
pub fn script_address(
	script_bytes: &[u8],
	network: NetworkIdKind,
	language: LanguageKind,
) -> Address {
	let script_hash = plutus_script_hash(script_bytes, language);
	EnterpriseAddress::new(
		network_id_kind_to_u8(network),
		&Credential::from_scripthash(&script_hash.into()),
	)
	.to_address()
}

pub fn payment_address(key_bytes: &[u8], network: NetworkIdKind) -> Address {
	let key_hash = sidechain_domain::crypto::blake2b(key_bytes);
	EnterpriseAddress::new(
		network_id_kind_to_u8(network),
		&Credential::from_keyhash(&key_hash.into()),
	)
	.to_address()
}

pub fn key_hash_address(pub_key_hash: &Ed25519KeyHash, network: NetworkIdKind) -> Address {
	EnterpriseAddress::new(network_id_kind_to_u8(network), &Credential::from_keyhash(pub_key_hash))
		.to_address()
}

pub fn ogmios_network_to_csl(network: ogmios_client::query_network::Network) -> NetworkIdKind {
	match network {
		ogmios_client::query_network::Network::Mainnet => NetworkIdKind::Mainnet,
		ogmios_client::query_network::Network::Testnet => NetworkIdKind::Testnet,
	}
}

fn network_id_kind_to_u8(network: NetworkIdKind) -> u8 {
	match network {
		NetworkIdKind::Mainnet => 1,
		NetworkIdKind::Testnet => 0,
	}
}

fn language_kind_to_u8(language: LanguageKind) -> u8 {
	match language {
		LanguageKind::PlutusV1 => 1,
		LanguageKind::PlutusV2 => 2,
		LanguageKind::PlutusV3 => 3,
	}
}

/// Creates a CSL [`TransactionBuilderConfig`] for given [`ProtocolParametersResponse`].
/// This function is not unit-testable because [`TransactionBuilderConfig`] has no public getters.
pub(crate) fn get_builder_config(
	context: &TransactionContext,
) -> Result<TransactionBuilderConfig, JsError> {
	let protocol_parameters = &context.protocol_parameters;
	TransactionBuilderConfigBuilder::new()
		.fee_algo(&linear_fee(protocol_parameters))
		.pool_deposit(&convert_value(&protocol_parameters.stake_pool_deposit)?.coin())
		.key_deposit(&convert_value(&protocol_parameters.stake_credential_deposit)?.coin())
		.max_value_size(protocol_parameters.max_value_size.bytes)
		.max_tx_size(protocol_parameters.max_transaction_size.bytes)
		.ex_unit_prices(&ExUnitPrices::new(
			&ratio_to_unit_interval(&protocol_parameters.script_execution_prices.memory),
			&ratio_to_unit_interval(&protocol_parameters.script_execution_prices.cpu),
		))
		.coins_per_utxo_byte(&protocol_parameters.min_utxo_deposit_coefficient.into())
		.build()
}

fn linear_fee(protocol_parameters: &ProtocolParametersResponse) -> LinearFee {
	let constant: BigNum = protocol_parameters.min_fee_constant.lovelace.into();
	LinearFee::new(&protocol_parameters.min_fee_coefficient.into(), &constant)
}

fn ratio_to_unit_interval(ratio: &fraction::Ratio<u64>) -> UnitInterval {
	UnitInterval::new(&(*ratio.numer()).into(), &(*ratio.denom()).into())
}

/// Coverts ogmios value to CSL value.
/// It could fail if the input contains negative values, for example ogmios values representing burn.
pub(crate) fn convert_value(value: &OgmiosValue) -> Result<Value, JsError> {
	if !value.native_tokens.is_empty() {
		let mut multiasset = MultiAsset::new();
		for (policy_id, assets) in value.native_tokens.iter() {
			let mut csl_assets = Assets::new();
			for asset in assets.iter() {
				let amount: u64 = asset.amount.try_into().map_err(|_| {
					JsError::from_str(&format!(
						"Could not convert Ogmios UTOX value, asset amount {} too large",
						asset.amount,
					))
				})?;
				let asset_name = AssetName::new(asset.name.clone()).map_err(|e| {
					JsError::from_str(&format!(
						"Could not convert Ogmios UTXO value, asset name is invalid: '{}'",
						e.to_string()
					))
				})?;
				csl_assets.insert(&asset_name, &amount.into());
			}
			multiasset.insert(&ScriptHash::from(*policy_id), &csl_assets);
		}
		Ok(Value::new_with_assets(&value.lovelace.into(), &multiasset))
	} else {
		Ok(Value::new(&value.lovelace.into()))
	}
}

/// Conversion of ogmios-client cost models to CSL
pub(crate) fn convert_cost_models(m: &PlutusCostModels) -> Costmdls {
	let mut mdls = Costmdls::new();
	mdls.insert(&Language::new_plutus_v1(), &CostModel::from(m.plutus_v1.to_owned()));
	mdls.insert(&Language::new_plutus_v2(), &CostModel::from(m.plutus_v2.to_owned()));
	mdls.insert(&Language::new_plutus_v3(), &CostModel::from(m.plutus_v3.to_owned()));
	mdls
}

/// Conversion of ogmios-client budget to CSL execution units
pub(crate) fn convert_ex_units(v: &OgmiosBudget) -> ExUnits {
	ExUnits::new(&v.memory.into(), &v.cpu.into())
}

pub(crate) fn empty_asset_name() -> AssetName {
	AssetName::new(vec![]).expect("Hardcoded empty asset name is valid")
}

pub(crate) trait OgmiosUtxoExt {
	fn to_csl_tx_input(&self) -> TransactionInput;
	fn to_csl_tx_output(&self) -> Result<TransactionOutput, JsError>;
	fn to_csl(&self) -> Result<TransactionUnspentOutput, JsError>;

	fn to_domain(&self) -> sidechain_domain::UtxoId;

	/// Encodes this UTXO as a nested constructor data format which is used by PC smart contracts
	fn to_uplc_plutus_data(&self) -> uplc::PlutusData;
}

impl OgmiosUtxoExt for OgmiosUtxo {
	fn to_csl_tx_input(&self) -> TransactionInput {
		TransactionInput::new(&TransactionHash::from(self.transaction.id), self.index.into())
	}

	fn to_csl_tx_output(&self) -> Result<TransactionOutput, JsError> {
		Ok(TransactionOutput::new(
			&Address::from_bech32(&self.address).map_err(|e| {
				JsError::from_str(&format!("Couldn't convert address from ogmios: '{}'", e))
			})?,
			&convert_value(&self.value)?,
		))
	}

	fn to_csl(&self) -> Result<TransactionUnspentOutput, JsError> {
		Ok(TransactionUnspentOutput::new(&self.to_csl_tx_input(), &self.to_csl_tx_output()?))
	}

	fn to_domain(&self) -> sidechain_domain::UtxoId {
		sidechain_domain::UtxoId {
			tx_hash: sidechain_domain::McTxHash(self.transaction.id),
			index: sidechain_domain::UtxoIndex(self.index),
		}
	}

	fn to_uplc_plutus_data(&self) -> uplc::PlutusData {
		datum_to_uplc_plutus_data(&self.to_domain())
	}
}

pub(crate) struct TransactionContext {
	/// This key is added as required signer and used to sign the transaction.
	pub(crate) payment_key: PrivateKey,
	/// Used to pay for the transaction fees and uncovered transaction inputs
	/// and as source of collateral inputs
	pub(crate) payment_utxos: Vec<OgmiosUtxo>,
	pub(crate) network: NetworkIdKind,
	pub(crate) protocol_parameters: ProtocolParametersResponse,
}

impl TransactionContext {
	pub(crate) fn payment_key_hash(&self) -> Ed25519KeyHash {
		self.payment_key.to_public().hash()
	}

	pub(crate) fn payment_address(&self) -> Address {
		key_hash_address(&self.payment_key.to_public().hash(), self.network)
	}

	pub(crate) fn sign(&self, tx: Transaction) -> Transaction {
		let tx_hash: [u8; 32] = sidechain_domain::crypto::blake2b(tx.body().to_bytes().as_ref());
		let signature = self.payment_key.sign(&tx_hash);
		let mut witness_set = tx.witness_set();
		let mut vkeywitnesses = witness_set.vkeys().unwrap_or_else(Vkeywitnesses::new);
		vkeywitnesses.add(&Vkeywitness::new(&Vkey::new(&self.payment_key.to_public()), &signature));
		witness_set.set_vkeys(&vkeywitnesses);
		Transaction::new(&tx.body(), &witness_set, tx.auxiliary_data())
	}
}

pub(crate) trait OgmiosUtxosExt {
	fn to_csl(&self) -> Result<TransactionUnspentOutputs, JsError>;
}

impl OgmiosUtxosExt for [OgmiosUtxo] {
	fn to_csl(&self) -> Result<TransactionUnspentOutputs, JsError> {
		let mut utxos = TransactionUnspentOutputs::new();
		for utxo in self {
			utxos.add(&utxo.to_csl()?);
		}
		Ok(utxos)
	}
}

pub(crate) trait TransactionBuilderExt {
	/// Creates output on the script address with datum that has 1 token with asset for the script and it has given datum attached.
	fn add_output_with_one_script_token(
		&mut self,
		validator: &PlutusScript,
		policy: &PlutusScript,
		datum: &PlutusData,
		ctx: &TransactionContext,
	) -> Result<(), JsError>;

	/// Adds ogmios inputs as collateral inputs to the tx builder.
	fn add_collateral_inputs(
		&mut self,
		ctx: &TransactionContext,
		inputs: &Vec<OgmiosUtxo>,
	) -> Result<(), JsError>;

	/// Adds minting of 1 token (with empty asset name) for the given script
	fn add_mint_one_script_token(
		&mut self,
		script: &PlutusScript,
		ex_units: ExUnits,
	) -> Result<(), JsError>;

	/// Sets fields required by the most of partner-chains smart contract transactions.
	/// Uses input from `ctx` to cover already present outputs.
	/// Adds collateral inputs using quite a simple algorithm.
	fn balance_update_and_build(
		&mut self,
		ctx: &TransactionContext,
	) -> Result<Transaction, JsError>;
}

impl TransactionBuilderExt for TransactionBuilder {
	fn add_output_with_one_script_token(
		&mut self,
		validator: &PlutusScript,
		policy: &PlutusScript,
		datum: &PlutusData,
		ctx: &TransactionContext,
	) -> Result<(), JsError> {
		let amount_builder = TransactionOutputBuilder::new()
			.with_address(&validator.address(ctx.network))
			.with_plutus_data(&datum)
			.next()?;
		let mut ma = MultiAsset::new();
		let mut assets = Assets::new();
		assets.insert(&empty_asset_name(), &1u64.into());
		ma.insert(&policy.script_hash().into(), &assets);
		let output = amount_builder.with_coin_and_asset(&0u64.into(), &ma).build()?;
		let min_ada = MinOutputAdaCalculator::new(
			&output,
			&DataCost::new_coins_per_byte(
				&ctx.protocol_parameters.min_utxo_deposit_coefficient.into(),
			),
		)
		.calculate_ada()?;
		let output = amount_builder.with_coin_and_asset(&min_ada, &ma).build()?;
		self.add_output(&output)
	}

	fn add_collateral_inputs(
		&mut self,
		ctx: &TransactionContext,
		inputs: &Vec<OgmiosUtxo>,
	) -> Result<(), JsError> {
		let mut collateral_builder = TxInputsBuilder::new();
		for utxo in inputs.iter() {
			collateral_builder.add_regular_input(
				&key_hash_address(&ctx.payment_key_hash(), ctx.network),
				&utxo.to_csl_tx_input(),
				&convert_value(&utxo.value)?,
			)?;
		}
		self.set_collateral(&collateral_builder);
		Ok(())
	}

	fn add_mint_one_script_token(
		&mut self,
		script: &PlutusScript,
		ex_units: ExUnits,
	) -> Result<(), JsError> {
		let mut mint_builder = MintBuilder::new();
		let validator_source = PlutusScriptSource::new(&script.to_csl());
		let mint_witness = MintWitness::new_plutus_script(
			&validator_source,
			&Redeemer::new(
				&RedeemerTag::new_mint(),
				&0u32.into(),
				&PlutusData::new_empty_constr_plutus_data(&0u32.into()),
				&ex_units,
			),
		);
		mint_builder.add_asset(&mint_witness, &empty_asset_name(), &Int::new_i32(1))?;
		self.set_mint_builder(&mint_builder);
		Ok(())
	}

	fn balance_update_and_build(
		&mut self,
		ctx: &TransactionContext,
	) -> Result<Transaction, JsError> {
		fn max_possible_collaterals(ctx: &TransactionContext) -> Vec<OgmiosUtxo> {
			let mut utxos = ctx.payment_utxos.clone();
			utxos.sort_by(|a, b| b.value.lovelace.cmp(&a.value.lovelace));
			let max_inputs = ctx.protocol_parameters.max_collateral_inputs;
			utxos
				.into_iter()
				.take(max_inputs.try_into().expect("max_collateral_input fit in usize"))
				.collect()
		}
		// Tries to balance tx with given collateral inputs
		fn try_balance(
			builder: &mut TransactionBuilder,
			collateral_inputs: &Vec<OgmiosUtxo>,
			ctx: &TransactionContext,
		) -> Result<Transaction, JsError> {
			builder.add_required_signer(&ctx.payment_key_hash());
			if collateral_inputs.is_empty() {
				builder.add_inputs_from_and_change(
					&ctx.payment_utxos.to_csl()?,
					CoinSelectionStrategyCIP2::LargestFirstMultiAsset,
					&ChangeConfig::new(&key_hash_address(&ctx.payment_key_hash(), ctx.network)),
				)?;
			} else {
				builder.add_collateral_inputs(ctx, &collateral_inputs)?;
				builder.set_script_data_hash(&[0u8; 32].into());
				// Fake script script data hash is required for proper fee computation
				builder.add_inputs_from_and_change_with_collateral_return(
					&ctx.payment_utxos.to_csl()?,
					CoinSelectionStrategyCIP2::LargestFirstMultiAsset,
					&ChangeConfig::new(&key_hash_address(&ctx.payment_key_hash(), ctx.network)),
					&ctx.protocol_parameters.collateral_percentage.into(),
				)?;
				builder.calc_script_data_hash(&convert_cost_models(
					&ctx.protocol_parameters.plutus_cost_models,
				))?;
			}
			builder.build_tx()
		}
		// Tries if the largest UTXO is enough to cover collateral, if not, adds more UTXOs
		// starting from the largest remaining.
		let mut selected = vec![];
		for input in max_possible_collaterals(ctx) {
			let mut builder = self.clone();
			// Check if the used inputs are enough
			let result = try_balance(&mut builder, &selected, ctx);
			if result.is_ok() {
				return result;
			}
			selected.push(input);
		}
		try_balance(self, &selected, ctx)
			.map_err(|e| JsError::from_str(&format!("Could not balance transaction. Usually it means that the payment key does not own UTXO set required to cover transaction outputs and fees or to provide collateral. Cause: {}", e)))
	}
}

pub(crate) trait InputsBuilderExt: Sized {
	fn add_script_utxo_input(
		&mut self,
		utxo: &OgmiosUtxo,
		script: &PlutusScript,
		ex_units: ExUnits,
	) -> Result<(), JsError>;

	/// Adds ogmios inputs to the tx inputs builder.
	fn add_key_inputs(&mut self, utxos: &[OgmiosUtxo], key: &Ed25519KeyHash)
		-> Result<(), JsError>;

	fn with_key_inputs(utxos: &[OgmiosUtxo], key: &Ed25519KeyHash) -> Result<Self, JsError>;
}

impl InputsBuilderExt for TxInputsBuilder {
	fn add_script_utxo_input(
		&mut self,
		utxo: &OgmiosUtxo,
		script: &PlutusScript,
		ex_units: ExUnits,
	) -> Result<(), JsError> {
		let input = utxo.to_csl_tx_input();
		let amount = convert_value(&utxo.value)?;
		let witness = PlutusWitness::new_without_datum(
			&script.to_csl(),
			&Redeemer::new(
				&RedeemerTag::new_spend(),
				// CSL will set redeemer index for the index of script input after sorting transaction inputs
				&0u32.into(),
				&PlutusData::new_empty_constr_plutus_data(&0u32.into()),
				&ex_units,
			),
		);
		self.add_plutus_script_input(&witness, &input, &amount);
		Ok(())
	}

	fn add_key_inputs(
		&mut self,
		utxos: &[OgmiosUtxo],
		key: &Ed25519KeyHash,
	) -> Result<(), JsError> {
		for utxo in utxos.iter() {
			self.add_key_input(key, &utxo.to_csl_tx_input(), &convert_value(&utxo.value)?);
		}
		Ok(())
	}

	fn with_key_inputs(utxos: &[OgmiosUtxo], key: &Ed25519KeyHash) -> Result<Self, JsError> {
		let mut tx_input_builder = Self::new();
		tx_input_builder.add_key_inputs(utxos, &key)?;
		Ok(tx_input_builder)
	}
}

#[cfg(test)]
mod tests {
	use super::payment_address;
	use crate::plutus_script::PlutusScript;
	use cardano_serialization_lib::LanguageKind::PlutusV2;
	use cardano_serialization_lib::{AssetName, Language, NetworkIdKind};
	use hex_literal::hex;
	use ogmios_client::{
		query_ledger_state::{PlutusCostModels, ProtocolParametersResponse, ScriptExecutionPrices},
		transactions::OgmiosBudget,
		types::{Asset, OgmiosBytesSize, OgmiosValue},
	};

	#[test]
	fn candidates_script_address_test() {
		let address = PlutusScript::from_cbor(
			&crate::plutus_script::tests::CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS,
			PlutusV2,
		)
		.address(NetworkIdKind::Testnet);
		assert_eq!(
			address.to_bech32(None).unwrap(),
			"addr_test1wq7vcwawqa29a5a2z7q8qs6k0cuvp6z2puvd8xx7vasuajq86paxz"
		);
	}

	#[test]
	fn payment_address_test() {
		let address = payment_address(
			&hex!("a35ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be7e3c5e88"),
			NetworkIdKind::Testnet,
		);
		assert_eq!(
			address.to_bech32(None).unwrap(),
			"addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy"
		)
	}

	#[test]
	fn linear_fee_test() {
		let fee = super::linear_fee(&test_protocol_parameters());
		assert_eq!(fee.constant(), 155381u32.into());
		assert_eq!(fee.coefficient(), 44u32.into());
	}

	#[test]
	fn ratio_to_unit_interval_test() {
		let ratio = fraction::Ratio::new(577, 10000);
		let unit_interval = super::ratio_to_unit_interval(&ratio);
		assert_eq!(unit_interval.numerator(), 577u64.into());
		assert_eq!(unit_interval.denominator(), 10000u64.into());
	}

	#[test]
	fn convert_value_without_multi_asset_test() {
		let ogmios_value = OgmiosValue::new_lovelace(1234567);
		let value = super::convert_value(&ogmios_value).unwrap();
		assert_eq!(value.coin(), 1234567u64.into());
		assert_eq!(value.multiasset(), None);
	}

	#[test]
	fn convert_value_with_multi_asset_test() {
		let ogmios_value = OgmiosValue {
			lovelace: 1234567,
			native_tokens: vec![
				([0u8; 28], vec![Asset { name: vec![], amount: 111 }]),
				(
					[1u8; 28],
					vec![
						Asset { name: hex!("222222").to_vec(), amount: 222 },
						Asset { name: hex!("333333").to_vec(), amount: 333 },
					],
				),
			]
			.into_iter()
			.collect(),
		};
		let value = super::convert_value(&ogmios_value).unwrap();
		assert_eq!(value.coin(), 1234567u64.into());
		let multiasset = value.multiasset().unwrap();
		assert_eq!(
			multiasset.get_asset(&[0u8; 28].into(), &AssetName::new(vec![]).unwrap()),
			111u64.into()
		);
		assert_eq!(
			multiasset
				.get_asset(&[1u8; 28].into(), &AssetName::new(hex!("222222").to_vec()).unwrap()),
			222u64.into()
		);
		assert_eq!(
			multiasset
				.get_asset(&[1u8; 28].into(), &AssetName::new(hex!("333333").to_vec()).unwrap()),
			333u64.into()
		);
	}

	#[test]
	fn convert_cost_models_test() {
		let cost_models =
			super::convert_cost_models(&test_protocol_parameters().plutus_cost_models);
		assert_eq!(cost_models.keys().len(), 3);
		assert_eq!(
			cost_models
				.get(&Language::new_plutus_v1())
				.unwrap()
				.get(0)
				.unwrap()
				.as_i32_or_nothing()
				.unwrap(),
			898148
		);
		assert_eq!(
			cost_models
				.get(&Language::new_plutus_v2())
				.unwrap()
				.get(1)
				.unwrap()
				.as_i32_or_nothing()
				.unwrap(),
			10
		);
		assert_eq!(
			cost_models
				.get(&Language::new_plutus_v3())
				.unwrap()
				.get(0)
				.unwrap()
				.as_i32_or_nothing()
				.unwrap(),
			-900
		);
	}

	#[test]
	fn convert_ex_values_test() {
		let ex_units = super::convert_ex_units(&OgmiosBudget { memory: 1000, cpu: 2000 });
		assert_eq!(ex_units.mem(), 1000u64.into());
		assert_eq!(ex_units.steps(), 2000u64.into());
	}

	fn test_protocol_parameters() -> ProtocolParametersResponse {
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
}

#[cfg(test)]
mod prop_tests {
	use super::{get_builder_config, OgmiosUtxoExt, TransactionBuilderExt, TransactionContext};
	use crate::test_values::*;
	use cardano_serialization_lib::{
		BigNum, ExUnits, NetworkIdKind, Transaction, TransactionBuilder, TransactionInputs,
		TransactionOutput, Value,
	};
	use ogmios_client::types::OgmiosValue;
	use ogmios_client::types::{OgmiosTx, OgmiosUtxo};
	use proptest::{
		array::uniform32,
		collection::{hash_set, vec},
		prelude::*,
	};
	use sidechain_domain::{McTxHash, UtxoId, UtxoIndex};

	const MIN_UTXO_LOVELACE: u64 = 1000000;
	const FIVE_ADA: u64 = 5000000;

	fn multi_asset_transaction_balancing_test(payment_utxos: Vec<OgmiosUtxo>) {
		let ctx = TransactionContext {
			payment_key: payment_key(),
			payment_utxos: payment_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		};
		let mut tx_builder = TransactionBuilder::new(&get_builder_config(&ctx).unwrap());
		tx_builder
			.add_mint_one_script_token(
				&test_policy(),
				ExUnits::new(&BigNum::zero(), &BigNum::zero()),
			)
			.unwrap();
		tx_builder
			.add_output_with_one_script_token(
				&test_validator(),
				&test_policy(),
				&test_plutus_data(),
				&ctx,
			)
			.unwrap();

		let tx = tx_builder.balance_update_and_build(&ctx).unwrap();

		used_inputs_lovelace_equals_outputs_and_fee(&tx, &payment_utxos);
		selected_collateral_inputs_equal_total_collateral_and_collateral_return(&tx, payment_utxos);
		fee_is_less_than_one_and_half_ada(&tx);
	}

	fn ada_only_transaction_balancing_test(payment_utxos: Vec<OgmiosUtxo>) {
		let ctx = TransactionContext {
			payment_key: payment_key(),
			payment_utxos: payment_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		};
		let mut tx_builder = TransactionBuilder::new(&get_builder_config(&ctx).unwrap());
		tx_builder
			.add_output(&TransactionOutput::new(&payment_addr(), &Value::new(&1500000u64.into())))
			.unwrap();

		let tx = tx_builder.balance_update_and_build(&ctx).unwrap();

		used_inputs_lovelace_equals_outputs_and_fee(&tx, &payment_utxos);
		there_is_no_collateral(&tx);
		fee_is_less_than_one_and_half_ada(&tx);
	}

	fn used_inputs_lovelace_equals_outputs_and_fee(
		tx: &Transaction,
		payment_utxos: &Vec<OgmiosUtxo>,
	) {
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

	fn selected_collateral_inputs_equal_total_collateral_and_collateral_return(
		tx: &Transaction,
		payment_utxos: Vec<OgmiosUtxo>,
	) {
		let collateral_inputs_sum: u64 =
			sum_lovelace(&match_inputs(&tx.body().collateral().unwrap(), &payment_utxos));
		let collateral_return: u64 = tx.body().collateral_return().unwrap().amount().coin().into();
		let total_collateral: u64 = tx.body().total_collateral().unwrap().into();
		assert_eq!(collateral_inputs_sum, collateral_return + total_collateral);
	}

	// Exact fee depends on inputs and outputs, but it definately is less than 1.5 ADA
	fn fee_is_less_than_one_and_half_ada(tx: &Transaction) {
		assert!(tx.body().fee() <= 1500000u64.into());
	}

	fn there_is_no_collateral(tx: &Transaction) {
		assert!(tx.body().total_collateral().is_none());
		assert!(tx.body().collateral_return().is_none());
		assert!(tx.body().collateral().is_none())
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

	fn sum_lovelace(utxos: &[OgmiosUtxo]) -> u64 {
		utxos.iter().map(|utxo| utxo.value.lovelace).sum()
	}

	proptest! {
		#[test]
		fn balance_tx_with_minted_token(payment_utxos in arb_payment_utxos(10)
			.prop_filter("Inputs total lovelace too low", |utxos| sum_lovelace(&utxos) > 4000000)) {
			multi_asset_transaction_balancing_test(payment_utxos)
		}

		#[test]
		fn balance_tx_with_ada_only_token(payment_utxos in arb_payment_utxos(10)
			.prop_filter("Inputs total lovelace too low", |utxos| sum_lovelace(&utxos) > 3000000)) {
			ada_only_transaction_balancing_test(payment_utxos)
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
