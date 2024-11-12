#![allow(dead_code)]

use crate::plutus_script::PlutusScript;
use cardano_serialization_lib::{
	Address, AssetName, Assets, BigNum, ChangeConfig, CostModel, Costmdls, Credential, DataCost,
	Ed25519KeyHash, EnterpriseAddress, ExUnitPrices, ExUnits, Int, JsError, Language, LanguageKind,
	LinearFee, MinOutputAdaCalculator, MintBuilder, MintWitness, MultiAsset, NetworkIdKind,
	PlutusData, PlutusScriptSource, PlutusWitness, Redeemer, RedeemerTag, ScriptHash, Transaction,
	TransactionBuilder, TransactionBuilderConfig, TransactionBuilderConfigBuilder, TransactionHash,
	TransactionInput, TransactionOutput, TransactionOutputBuilder, TransactionUnspentOutput,
	TransactionUnspentOutputs, TxInputsBuilder, UnitInterval, Value,
};
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
}

pub(crate) struct TransactionContext {
	pub(crate) payment_key_hash: Ed25519KeyHash,
	pub(crate) collaterals: Vec<OgmiosUtxo>,
	pub(crate) payment_utxos: Vec<OgmiosUtxo>,
	pub(crate) network: NetworkIdKind,
	pub(crate) protocol_parameters: ProtocolParametersResponse,
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
		script: &PlutusScript,
		datum: &PlutusData,
		ctx: &TransactionContext,
	) -> Result<(), JsError>;

	/// Adds ogmios inputs as collateral inputs to the tx builder.
	fn add_collateral_inputs(&mut self, ctx: &TransactionContext) -> Result<(), JsError>;

	/// Adds minting of 1 token (with empty asset name) for the given script
	fn add_mint_one_script_token(
		&mut self,
		script: &PlutusScript,
		ex_units: ExUnits,
	) -> Result<(), JsError>;

	/// Sets fields required by the most of partner-chains smart contract transactions
	fn set_required_fields_and_build(
		&mut self,
		ctx: &TransactionContext,
	) -> Result<Transaction, JsError>;
}

impl TransactionBuilderExt for TransactionBuilder {
	fn add_output_with_one_script_token(
		&mut self,
		script: &PlutusScript,
		datum: &PlutusData,
		ctx: &TransactionContext,
	) -> Result<(), JsError> {
		let amount_builder = TransactionOutputBuilder::new()
			.with_address(&script.address(ctx.network))
			.with_plutus_data(&datum)
			.next()?;
		let mut ma = MultiAsset::new();
		let mut assets = Assets::new();
		assets.insert(&empty_asset_name(), &1u64.into());
		ma.insert(&script.script_hash().into(), &assets);
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

	fn add_collateral_inputs(&mut self, ctx: &TransactionContext) -> Result<(), JsError> {
		let mut collateral_builder = TxInputsBuilder::new();
		collateral_builder.add_key_inputs(&ctx.collaterals, &ctx.payment_key_hash)?;
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

	fn set_required_fields_and_build(
		&mut self,
		ctx: &TransactionContext,
	) -> Result<Transaction, JsError> {
		self.add_collateral_inputs(ctx)?;
		self.calc_script_data_hash(&convert_cost_models(
			&ctx.protocol_parameters.plutus_cost_models,
		))?;
		self.add_required_signer(&ctx.payment_key_hash);
		self.add_inputs_from_and_change_with_collateral_return(
			&ctx.payment_utxos.to_csl()?,
			cardano_serialization_lib::CoinSelectionStrategyCIP2::LargestFirstMultiAsset,
			&ChangeConfig::new(&key_hash_address(&ctx.payment_key_hash, ctx.network)),
			&ctx.protocol_parameters.collateral_percentage.into(),
		)?;
		self.build_tx()
	}
}

pub(crate) trait InputsBuilderExt {
	fn add_script_utxo_input(
		&mut self,
		utxo: &OgmiosUtxo,
		script: &PlutusScript,
		ex_units: ExUnits,
	) -> Result<(), JsError>;

	/// Adds ogmios inputs to the tx inputs builder.
	fn add_key_inputs(&mut self, utxos: &[OgmiosUtxo], key: &Ed25519KeyHash)
		-> Result<(), JsError>;
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
