use crate::plutus_script::PlutusScript;
use cardano_serialization_lib::*;
use fraction::{FromPrimitive, Ratio};
use ogmios_client::query_ledger_state::ReferenceScriptsCosts;
use ogmios_client::{
	query_ledger_state::{PlutusCostModels, ProtocolParametersResponse, QueryLedgerState},
	query_network::QueryNetwork,
	transactions::{OgmiosBudget, OgmiosEvaluateTransactionResponse},
	types::{OgmiosUtxo, OgmiosValue},
};
use sidechain_domain::{AssetId, MainchainAddressHash, MainchainPrivateKey, NetworkType, UtxoId};

pub(crate) fn plutus_script_hash(script_bytes: &[u8], language: Language) -> [u8; 28] {
	// Before hashing the script, we need to prepend with byte denoting the language.
	let mut buf: Vec<u8> = vec![language_to_u8(language)];
	buf.extend(script_bytes);
	sidechain_domain::crypto::blake2b(buf.as_slice())
}

/// Builds an CSL `Address` for plutus script from the data obtained from smart contracts.
pub fn script_address(script_bytes: &[u8], network: NetworkIdKind, language: Language) -> Address {
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

pub trait NetworkTypeExt {
	fn to_csl(&self) -> NetworkIdKind;
}
impl NetworkTypeExt for NetworkType {
	fn to_csl(&self) -> NetworkIdKind {
		match self {
			Self::Mainnet => NetworkIdKind::Mainnet,
			Self::Testnet => NetworkIdKind::Testnet,
		}
	}
}

fn network_id_kind_to_u8(network: NetworkIdKind) -> u8 {
	match network {
		NetworkIdKind::Mainnet => 1,
		NetworkIdKind::Testnet => 0,
	}
}

fn language_to_u8(language: Language) -> u8 {
	match language.kind() {
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
		.pool_deposit(&protocol_parameters.stake_pool_deposit.to_csl()?.coin())
		.key_deposit(&protocol_parameters.stake_credential_deposit.to_csl()?.coin())
		.max_value_size(protocol_parameters.max_value_size.bytes)
		.max_tx_size(protocol_parameters.max_transaction_size.bytes)
		.ex_unit_prices(&ExUnitPrices::new(
			&ratio_to_unit_interval(&protocol_parameters.script_execution_prices.memory),
			&ratio_to_unit_interval(&protocol_parameters.script_execution_prices.cpu),
		))
		.coins_per_utxo_byte(&protocol_parameters.min_utxo_deposit_coefficient.into())
		.ref_script_coins_per_byte(&convert_reference_script_costs(
			&protocol_parameters.min_fee_reference_scripts.clone(),
		)?)
		.build()
}

fn linear_fee(protocol_parameters: &ProtocolParametersResponse) -> LinearFee {
	let constant: BigNum = protocol_parameters.min_fee_constant.lovelace.into();
	LinearFee::new(&protocol_parameters.min_fee_coefficient.into(), &constant)
}

fn ratio_to_unit_interval(ratio: &fraction::Ratio<u64>) -> UnitInterval {
	UnitInterval::new(&(*ratio.numer()).into(), &(*ratio.denom()).into())
}

pub trait OgmiosValueExt {
	fn to_csl(&self) -> Result<Value, JsError>;
}

impl OgmiosValueExt for OgmiosValue {
	/// Coverts ogmios value to CSL value.
	/// It could fail if the input contains negative values, for example ogmios values representing burn.
	fn to_csl(&self) -> Result<Value, JsError> {
		if !self.native_tokens.is_empty() {
			let mut multiasset = MultiAsset::new();
			for (policy_id, assets) in self.native_tokens.iter() {
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
							e
						))
					})?;
					csl_assets.insert(&asset_name, &amount.into());
				}
				multiasset.insert(&ScriptHash::from(*policy_id), &csl_assets);
			}
			Ok(Value::new_with_assets(&self.lovelace.into(), &multiasset))
		} else {
			Ok(Value::new(&self.lovelace.into()))
		}
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

pub(crate) fn convert_reference_script_costs(
	costs: &ReferenceScriptsCosts,
) -> Result<UnitInterval, JsError> {
	let r = Ratio::<u64>::from_f64(costs.base).ok_or_else(|| {
		JsError::from_str(&format!("Failed to decode cost base {} as a u64 ratio", costs.base))
	})?;
	let numerator = BigNum::from(*r.numer());
	let denominator = BigNum::from(*r.denom());
	Ok(UnitInterval::new(&numerator, &denominator))
}

/// Returns the budget of the first validator as [`ExUnits`]
pub(crate) fn get_first_validator_budget(
	validators_budgets: Vec<OgmiosEvaluateTransactionResponse>,
) -> Result<ExUnits, JsError> {
	let validator_budget = validators_budgets.first().ok_or_else(|| {
		JsError::from_str("Internal error: cannot use evaluateTransaction response")
	})?;
	Ok(convert_ex_units(&validator_budget.budget))
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScriptExUnits {
	pub mint_ex_units: Vec<ExUnits>,
	pub spend_ex_units: Vec<ExUnits>,
}

impl ScriptExUnits {
	pub fn new() -> Self {
		ScriptExUnits { mint_ex_units: Vec::new(), spend_ex_units: Vec::new() }
	}
	pub fn with_mint_ex_units(self, mint_ex_units: Vec<ExUnits>) -> Self {
		Self { mint_ex_units, ..self }
	}
	pub fn with_spend_ex_units(self, spend_ex_units: Vec<ExUnits>) -> Self {
		Self { spend_ex_units, ..self }
	}
}

pub(crate) fn get_validator_budgets(
	mut responses: Vec<OgmiosEvaluateTransactionResponse>,
) -> ScriptExUnits {
	responses.sort_by_key(|r| r.validator.index);
	let (mint_ex_units, spend_ex_units) = responses
		.into_iter()
		.partition::<Vec<_>, _>(|response| response.validator.purpose == "mint");
	let mint_ex_units = mint_ex_units.into_iter().map(ex_units_from_response).collect();
	let spend_ex_units = spend_ex_units.into_iter().map(ex_units_from_response).collect();

	ScriptExUnits { mint_ex_units, spend_ex_units }
}

fn ex_units_from_response(resp: OgmiosEvaluateTransactionResponse) -> ExUnits {
	ExUnits::new(&resp.budget.memory.into(), &resp.budget.cpu.into())
}

/// Conversion of ogmios-client budget to CSL execution units
pub(crate) fn convert_ex_units(v: &OgmiosBudget) -> ExUnits {
	ExUnits::new(&v.memory.into(), &v.cpu.into())
}

pub(crate) fn empty_asset_name() -> AssetName {
	AssetName::new(vec![]).expect("Hardcoded empty asset name is valid")
}

pub fn zero_ex_units() -> ExUnits {
	ExUnits::new(&BigNum::zero(), &BigNum::zero())
}

pub(crate) trait OgmiosUtxoExt {
	fn to_csl_tx_input(&self) -> TransactionInput;
	fn to_csl_tx_output(&self) -> Result<TransactionOutput, JsError>;
	fn to_csl(&self) -> Result<TransactionUnspentOutput, JsError>;

	fn to_domain(&self) -> sidechain_domain::UtxoId;

	fn get_asset_amount(&self, asset: &AssetId) -> i128;
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
			&self.value.to_csl()?,
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

	fn get_asset_amount(&self, asset_id: &AssetId) -> i128 {
		self.value
			.native_tokens
			.get(&asset_id.policy_id.0)
			.cloned()
			.unwrap_or_default()
			.iter()
			.find(|asset| asset.name == asset_id.asset_name.0.to_vec())
			.map_or_else(|| 0, |asset| asset.amount)
	}
}

pub trait UtxoIdExt {
	fn to_csl(&self) -> TransactionInput;
}

impl UtxoIdExt for UtxoId {
	fn to_csl(&self) -> TransactionInput {
		TransactionInput::new(&TransactionHash::from(self.tx_hash.0), self.index.0.into())
	}
}

pub trait MainchainPrivateKeyExt {
	fn to_pub_key_hash(&self) -> MainchainAddressHash;
}

impl MainchainPrivateKeyExt for MainchainPrivateKey {
	fn to_pub_key_hash(&self) -> MainchainAddressHash {
		cardano_serialization_lib::PrivateKey::from_normal_bytes(&self.0)
			.expect("Conversion cannot fail on valid MainchainPrivateKey values")
			.to_public()
			.hash()
			.to_bytes()
			.as_slice()
			.try_into()
			.map(MainchainAddressHash)
			.expect("Conversion cannot fail as representation is the same")
	}
}

pub(crate) struct TransactionContext {
	/// This key is added as required signer and used to sign the transaction.
	pub(crate) payment_key: PrivateKey,
	/// Used to pay for the transaction fees and uncovered transaction inputs
	/// and as source of collateral inputs
	pub(crate) payment_key_utxos: Vec<OgmiosUtxo>,
	pub(crate) network: NetworkIdKind,
	pub(crate) protocol_parameters: ProtocolParametersResponse,
}

impl TransactionContext {
	/// Gets `TransactionContext`, having UTXOs for the given payment key and the network configuration,
	/// required to perform most of the partner-chains smart contract operations.
	pub(crate) async fn for_payment_key<C: QueryLedgerState + QueryNetwork>(
		payment_signing_key: [u8; 32],
		client: &C,
	) -> Result<TransactionContext, anyhow::Error> {
		let payment_key = PrivateKey::from_normal_bytes(&payment_signing_key)?;
		let network = client.shelley_genesis_configuration().await?.network.to_csl();
		let protocol_parameters = client.query_protocol_parameters().await?;
		let payment_address = key_hash_address(&payment_key.to_public().hash(), network);
		let payment_key_utxos = client.query_utxos(&[payment_address.to_bech32(None)?]).await?;
		Ok(TransactionContext { payment_key, payment_key_utxos, network, protocol_parameters })
	}

	pub(crate) fn payment_key_hash(&self) -> Ed25519KeyHash {
		self.payment_key.to_public().hash()
	}

	pub(crate) fn sign(&self, tx: &Transaction) -> Transaction {
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
		inputs: &[OgmiosUtxo],
	) -> Result<(), JsError>;

	/// Adds minting of 1 token (with empty asset name) for the given script
	fn add_mint_one_script_token(
		&mut self,
		script: &PlutusScript,
		ex_units: &ExUnits,
	) -> Result<(), JsError>;

	/// Adds minting of 1 token (with empty asset name) for the given script using reference input
	fn add_mint_script_token_using_reference_script(
		&mut self,
		script: &PlutusScript,
		ref_input: &TransactionInput,
		amount: &BigNum,
		ex_units: &ExUnits,
	) -> Result<(), JsError>;

	fn add_mint_one_script_token_using_reference_script(
		&mut self,
		script: &PlutusScript,
		ref_input: &TransactionInput,
		ex_units: &ExUnits,
	) -> Result<(), JsError> {
		self.add_mint_script_token_using_reference_script(script, ref_input, &1u64.into(), ex_units)
	}

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
			.with_plutus_data(datum)
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
		inputs: &[OgmiosUtxo],
	) -> Result<(), JsError> {
		let mut collateral_builder = TxInputsBuilder::new();
		for utxo in inputs.iter() {
			collateral_builder.add_regular_input(
				&key_hash_address(&ctx.payment_key_hash(), ctx.network),
				&utxo.to_csl_tx_input(),
				&utxo.value.to_csl()?,
			)?;
		}
		self.set_collateral(&collateral_builder);
		Ok(())
	}

	fn add_mint_one_script_token(
		&mut self,
		script: &PlutusScript,
		ex_units: &ExUnits,
	) -> Result<(), JsError> {
		let mut mint_builder = self.get_mint_builder().unwrap_or(MintBuilder::new());

		let validator_source = PlutusScriptSource::new(&script.to_csl());
		let mint_witness = MintWitness::new_plutus_script(
			&validator_source,
			&Redeemer::new(
				&RedeemerTag::new_mint(),
				&0u32.into(),
				&PlutusData::new_empty_constr_plutus_data(&0u32.into()),
				ex_units,
			),
		);
		mint_builder.add_asset(&mint_witness, &empty_asset_name(), &Int::new_i32(1))?;
		self.set_mint_builder(&mint_builder);
		Ok(())
	}

	fn add_mint_script_token_using_reference_script(
		&mut self,
		script: &PlutusScript,
		ref_input: &TransactionInput,
		amount: &BigNum,
		ex_units: &ExUnits,
	) -> Result<(), JsError> {
		let mut mint_builder = self.get_mint_builder().unwrap_or(MintBuilder::new());

		let validator_source = PlutusScriptSource::new_ref_input(
			&script.csl_script_hash(),
			ref_input,
			&script.language,
			script.bytes.len(),
		);
		let mint_witness = MintWitness::new_plutus_script(
			&validator_source,
			&Redeemer::new(
				&RedeemerTag::new_mint(),
				&0u32.into(),
				&PlutusData::new_empty_constr_plutus_data(&0u32.into()),
				ex_units,
			),
		);
		mint_builder.add_asset(&mint_witness, &empty_asset_name(), &Int::new(amount))?;
		self.set_mint_builder(&mint_builder);
		Ok(())
	}

	fn balance_update_and_build(
		&mut self,
		ctx: &TransactionContext,
	) -> Result<Transaction, JsError> {
		fn max_possible_collaterals(ctx: &TransactionContext) -> Vec<OgmiosUtxo> {
			let mut utxos = ctx.payment_key_utxos.clone();
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
			collateral_inputs: &[OgmiosUtxo],
			ctx: &TransactionContext,
		) -> Result<Transaction, JsError> {
			builder.add_required_signer(&ctx.payment_key_hash());
			if collateral_inputs.is_empty() {
				builder.add_inputs_from_and_change(
					&ctx.payment_key_utxos.to_csl()?,
					CoinSelectionStrategyCIP2::LargestFirstMultiAsset,
					&ChangeConfig::new(&key_hash_address(&ctx.payment_key_hash(), ctx.network)),
				)?;
			} else {
				builder.add_collateral_inputs(ctx, collateral_inputs)?;
				builder.set_script_data_hash(&[0u8; 32].into());
				// Fake script script data hash is required for proper fee computation
				builder.add_inputs_from_and_change_with_collateral_return(
					&ctx.payment_key_utxos.to_csl()?,
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

pub(crate) trait TransactionOutputAmountBuilderExt: Sized {
	fn get_minimum_ada(&self, ctx: &TransactionContext) -> Result<BigNum, JsError>;
	fn with_minimum_ada(self, ctx: &TransactionContext) -> Result<Self, JsError>;
	fn with_minimum_ada_and_asset(
		self,
		ma: &MultiAsset,
		ctx: &TransactionContext,
	) -> Result<Self, JsError>;
}

impl TransactionOutputAmountBuilderExt for TransactionOutputAmountBuilder {
	fn get_minimum_ada(&self, ctx: &TransactionContext) -> Result<BigNum, JsError> {
		MinOutputAdaCalculator::new(
			&self.build()?,
			&DataCost::new_coins_per_byte(
				&ctx.protocol_parameters.min_utxo_deposit_coefficient.into(),
			),
		)
		.calculate_ada()
	}

	fn with_minimum_ada(self, ctx: &TransactionContext) -> Result<Self, JsError> {
		let min_ada = self.with_coin(&0u64.into()).get_minimum_ada(ctx)?;
		Ok(self.with_coin(&min_ada))
	}

	fn with_minimum_ada_and_asset(
		self,
		ma: &MultiAsset,
		ctx: &TransactionContext,
	) -> Result<Self, JsError> {
		let min_ada = self.with_coin_and_asset(&0u64.into(), ma).get_minimum_ada(ctx)?;
		Ok(self.with_coin_and_asset(&min_ada, &ma))
	}
}

pub(crate) trait InputsBuilderExt: Sized {
	fn add_script_utxo_input(
		&mut self,
		utxo: &OgmiosUtxo,
		script: &PlutusScript,
		ex_units: &ExUnits,
	) -> Result<(), JsError>;

	fn add_script_utxo_input_with_data(
		&mut self,
		utxo: &OgmiosUtxo,
		script: &PlutusScript,
		data: &PlutusData,
		ex_units: &ExUnits,
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
		ex_units: &ExUnits,
	) -> Result<(), JsError> {
		self.add_script_utxo_input_with_data(
			utxo,
			script,
			&PlutusData::new_empty_constr_plutus_data(&0u32.into()),
			ex_units,
		)
	}

	fn add_script_utxo_input_with_data(
		&mut self,
		utxo: &OgmiosUtxo,
		script: &PlutusScript,
		data: &PlutusData,
		ex_units: &ExUnits,
	) -> Result<(), JsError> {
		let input = utxo.to_csl_tx_input();
		let amount = &utxo.value.to_csl()?;
		let witness = PlutusWitness::new_without_datum(
			&script.to_csl(),
			&Redeemer::new(
				&RedeemerTag::new_spend(),
				// CSL will set redeemer index for the index of script input after sorting transaction inputs
				&0u32.into(),
				data,
				ex_units,
			),
		);
		self.add_plutus_script_input(&witness, &input, amount);
		Ok(())
	}

	fn add_key_inputs(
		&mut self,
		utxos: &[OgmiosUtxo],
		key: &Ed25519KeyHash,
	) -> Result<(), JsError> {
		for utxo in utxos.iter() {
			self.add_key_input(key, &utxo.to_csl_tx_input(), &utxo.value.to_csl()?);
		}
		Ok(())
	}

	fn with_key_inputs(utxos: &[OgmiosUtxo], key: &Ed25519KeyHash) -> Result<Self, JsError> {
		let mut tx_input_builder = Self::new();
		tx_input_builder.add_key_inputs(utxos, key)?;
		Ok(tx_input_builder)
	}
}

pub(crate) trait AssetNameExt {
	fn to_csl(&self) -> Result<cardano_serialization_lib::AssetName, JsError>;
}

impl AssetNameExt for sidechain_domain::AssetName {
	fn to_csl(&self) -> Result<cardano_serialization_lib::AssetName, JsError> {
		cardano_serialization_lib::AssetName::new(self.0.to_vec())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::plutus_script::PlutusScript;
	use crate::test_values::protocol_parameters;
	use cardano_serialization_lib::{AssetName, Language, NetworkIdKind};
	use hex_literal::hex;
	use ogmios_client::{
		transactions::{OgmiosBudget, OgmiosValidatorIndex},
		types::{Asset, OgmiosValue},
	};
	use pretty_assertions::assert_eq;

	#[test]
	fn candidates_script_address_test() {
		let address = PlutusScript::from_cbor(
			&crate::plutus_script::tests::CANDIDATES_SCRIPT_WITH_APPLIED_PARAMS,
			Language::new_plutus_v2(),
		)
		.address(NetworkIdKind::Testnet);
		assert_eq!(
			address.to_bech32(None).unwrap(),
			"addr_test1wp6t6apkj6kdz6j0jmtjqc5887cnrnfw9rdpressk3ak66sf6h0hm"
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
		let fee = super::linear_fee(&protocol_parameters());
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
		let value = &ogmios_value.to_csl().unwrap();
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
		let value = &ogmios_value.to_csl().unwrap();
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
		let cost_models = super::convert_cost_models(&protocol_parameters().plutus_cost_models);
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

	#[test]
	fn get_validator_budgets_works() {
		let result = get_validator_budgets(vec![
			OgmiosEvaluateTransactionResponse {
				validator: OgmiosValidatorIndex::new(1, "mint"),
				budget: OgmiosBudget::new(11, 21),
			},
			OgmiosEvaluateTransactionResponse {
				validator: OgmiosValidatorIndex::new(0, "spend"),
				budget: OgmiosBudget::new(10, 20),
			},
			OgmiosEvaluateTransactionResponse {
				validator: OgmiosValidatorIndex::new(3, "mint"),
				budget: OgmiosBudget::new(13, 23),
			},
			OgmiosEvaluateTransactionResponse {
				validator: OgmiosValidatorIndex::new(2, "spend"),
				budget: OgmiosBudget::new(12, 22),
			},
		]);

		let expected = ScriptExUnits {
			mint_ex_units: vec![
				ExUnits::new(&11u64.into(), &21u64.into()),
				ExUnits::new(&13u64.into(), &23u64.into()),
			],
			spend_ex_units: vec![
				ExUnits::new(&10u64.into(), &20u64.into()),
				ExUnits::new(&12u64.into(), &22u64.into()),
			],
		};

		assert_eq!(result, expected);
	}
}

#[cfg(test)]
mod prop_tests {
	use super::{
		get_builder_config, zero_ex_units, OgmiosUtxoExt, TransactionBuilderExt, TransactionContext,
	};
	use crate::test_values::*;
	use cardano_serialization_lib::{
		NetworkIdKind, Transaction, TransactionBuilder, TransactionInputs, TransactionOutput, Value,
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
			payment_key_utxos: payment_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		};
		let mut tx_builder = TransactionBuilder::new(&get_builder_config(&ctx).unwrap());
		tx_builder.add_mint_one_script_token(&test_policy(), &zero_ex_units()).unwrap();
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
			payment_key_utxos: payment_utxos.clone(),
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
			.prop_filter("Inputs total lovelace too low", |utxos| sum_lovelace(utxos) > 4000000)) {
			multi_asset_transaction_balancing_test(payment_utxos)
		}

		#[test]
		fn balance_tx_with_ada_only_token(payment_utxos in arb_payment_utxos(10)
			.prop_filter("Inputs total lovelace too low", |utxos| sum_lovelace(utxos) > 3000000)) {
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
				address: PAYMENT_ADDR.into(),
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
