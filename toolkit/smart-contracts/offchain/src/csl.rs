use crate::cardano_keys::CardanoPaymentSigningKey;
use crate::plutus_script::PlutusScript;
use cardano_serialization_lib::*;
use fraction::{FromPrimitive, Ratio};
use ogmios_client::query_ledger_state::ReferenceScriptsCosts;
use ogmios_client::transactions::Transactions;
use ogmios_client::{
	query_ledger_state::{PlutusCostModels, ProtocolParametersResponse, QueryLedgerState},
	query_network::QueryNetwork,
	transactions::OgmiosEvaluateTransactionResponse,
	types::{OgmiosUtxo, OgmiosValue},
};
use sidechain_domain::{AssetId, NetworkType, UtxoId};
use std::collections::HashMap;

/// Constructs [Transaction] from CBOR bytes
pub fn transaction_from_bytes(cbor: Vec<u8>) -> anyhow::Result<Transaction> {
	Transaction::from_bytes(cbor).map_err(|e| anyhow::anyhow!(e))
}

/// Constructs [Vkeywitness] from CBOR bytes
pub fn vkey_witness_from_bytes(cbor: Vec<u8>) -> anyhow::Result<Vkeywitness> {
	Vkeywitness::from_bytes(cbor).map_err(|e| anyhow::anyhow!(e))
}

pub(crate) fn plutus_script_hash(script_bytes: &[u8], language: Language) -> [u8; 28] {
	// Before hashing the script, we need to prepend with byte denoting the language.
	let mut buf: Vec<u8> = vec![language_to_u8(language)];
	buf.extend(script_bytes);
	sidechain_domain::crypto::blake2b(buf.as_slice())
}

/// Builds a CSL [Address] for plutus script from the data obtained from smart contracts.
pub fn script_address(script_bytes: &[u8], network: NetworkIdKind, language: Language) -> Address {
	let script_hash = plutus_script_hash(script_bytes, language);
	EnterpriseAddress::new(
		network_id_kind_to_u8(network),
		&Credential::from_scripthash(&script_hash.into()),
	)
	.to_address()
}

/// Builds a CSL [Address] for the specified network from Cardano verification key bytes.
pub fn payment_address(cardano_verification_key_bytes: &[u8], network: NetworkIdKind) -> Address {
	let key_hash = sidechain_domain::crypto::blake2b(cardano_verification_key_bytes);
	EnterpriseAddress::new(
		network_id_kind_to_u8(network),
		&Credential::from_keyhash(&key_hash.into()),
	)
	.to_address()
}

/// Builds a CSL [Address] for the specified network from a Cardano verification key hash.
pub fn key_hash_address(pub_key_hash: &Ed25519KeyHash, network: NetworkIdKind) -> Address {
	EnterpriseAddress::new(network_id_kind_to_u8(network), &Credential::from_keyhash(pub_key_hash))
		.to_address()
}

/// Extension trait for [NetworkType].
pub trait NetworkTypeExt {
	/// Converts [NetworkType] to CSL [cardano_serialization_lib::Value].
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
		.deduplicate_explicit_ref_inputs_with_regular_inputs(true)
		.build()
}

fn linear_fee(protocol_parameters: &ProtocolParametersResponse) -> LinearFee {
	let constant: BigNum = protocol_parameters.min_fee_constant.lovelace.into();
	LinearFee::new(&protocol_parameters.min_fee_coefficient.into(), &constant)
}

fn ratio_to_unit_interval(ratio: &fraction::Ratio<u64>) -> UnitInterval {
	UnitInterval::new(&(*ratio.numer()).into(), &(*ratio.denom()).into())
}

/// Extension trait for [OgmiosValue].
pub trait OgmiosValueExt {
	/// Converts [OgmiosValue] to CSL [cardano_serialization_lib::Value].
	/// It can fail if the input contains negative values, for example Ogmios values representing burn.
	fn to_csl(&self) -> Result<Value, JsError>;
}

impl OgmiosValueExt for OgmiosValue {
	fn to_csl(&self) -> Result<Value, JsError> {
		if !self.native_tokens.is_empty() {
			let mut multiasset = MultiAsset::new();
			for (policy_id, assets) in self.native_tokens.iter() {
				let mut csl_assets = Assets::new();
				for asset in assets.iter() {
					let asset_name = AssetName::new(asset.name.clone()).map_err(|e| {
						JsError::from_str(&format!(
							"Could not convert Ogmios UTXO value, asset name is invalid: '{}'",
							e
						))
					})?;
					csl_assets.insert(&asset_name, &asset.amount.into());
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

fn ex_units_from_response(resp: OgmiosEvaluateTransactionResponse) -> ExUnits {
	ExUnits::new(&resp.budget.memory.into(), &resp.budget.cpu.into())
}

/// Type representing transaction execution costs.
pub(crate) enum Costs {
	/// Zero costs. Used as a dummy value when submitting a transaction for cost calculation.
	ZeroCosts,
	/// Variant containing actual costs.
	Costs {
		/// Mapping script hashes to minting policy execution costs.
		mints: HashMap<cardano_serialization_lib::ScriptHash, ExUnits>,
		/// Mapping spend indices to validator script execution costs.
		spends: HashMap<u32, ExUnits>,
	},
}

/// Interface for retrieving execution costs.
pub(crate) trait CostStore {
	/// Returns [ExUnits] cost of a minting policy for a given [PlutusScript].
	fn get_mint(&self, script: &PlutusScript) -> ExUnits;
	/// Returns [ExUnits] cost of a validator script for a given spend index.
	fn get_spend(&self, spend_ix: u32) -> ExUnits;
	/// Returns spend cost of the single validator script in a transaction.
	/// It panics if there is not exactly one validator script execution.
	fn get_one_spend(&self) -> ExUnits;
	/// Returns indices of validator scripts as they appear in the CSL transaction.
	/// These indices can be used in conjunction with [get_spend].
	fn get_spend_indices(&self) -> Vec<u32>;
}

impl CostStore for Costs {
	fn get_mint(&self, script: &PlutusScript) -> ExUnits {
		match self {
			Costs::ZeroCosts => zero_ex_units(),
			Costs::Costs { mints, .. } => mints
				.get(&script.csl_script_hash())
				.expect("get_mint should not be called with an unknown script")
				.clone(),
		}
	}
	fn get_spend(&self, spend_ix: u32) -> ExUnits {
		match self {
			Costs::ZeroCosts => zero_ex_units(),
			Costs::Costs { spends, .. } => spends
				.get(&spend_ix)
				.expect("get_spend should not be called with an unknown spend index")
				.clone(),
		}
	}
	fn get_one_spend(&self) -> ExUnits {
		match self {
			Costs::ZeroCosts => zero_ex_units(),
			Costs::Costs { spends, .. } => match spends.values().collect::<Vec<_>>()[..] {
				[x] => x.clone(),
				_ => panic!(
					"get_one_spend should only be called when exactly one spend is expected to be present"
				),
			},
		}
	}
	fn get_spend_indices(&self) -> Vec<u32> {
		match self {
			Costs::ZeroCosts => vec![],
			Costs::Costs { spends, .. } => spends.keys().cloned().collect(),
		}
	}
}

impl Costs {
	#[cfg(test)]
	/// Constructs new [Costs] with given `mints` and `spends`.
	pub(crate) fn new(
		mints: HashMap<cardano_serialization_lib::ScriptHash, ExUnits>,
		spends: HashMap<u32, ExUnits>,
	) -> Costs {
		Costs::Costs { mints, spends }
	}

	/// Creates a [Transaction] with correctly set script execution costs.
	///
	/// Arguments:
	///  - `make_tx`: A function that takes a [Costs] value, and returns a [anyhow::Result<Transaction>].
	///               This function is meant to describe which execution cost is used where in the transaction.
	///  - `client`: Ogmios client
	pub(crate) async fn calculate_costs<T: Transactions, F>(
		make_tx: F,
		client: &T,
	) -> anyhow::Result<Transaction>
	where
		F: Fn(Costs) -> anyhow::Result<Transaction>,
	{
		// This double evaluation is needed to correctly set costs in some cases.
		let tx = make_tx(Costs::ZeroCosts)?;
		// stage 1
		let costs = Self::from_ogmios(&tx, client).await?;

		let tx = make_tx(costs)?;
		// stage 2
		let costs = Self::from_ogmios(&tx, client).await?;

		make_tx(costs)
	}

	async fn from_ogmios<T: Transactions>(tx: &Transaction, client: &T) -> anyhow::Result<Costs> {
		let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await?;

		let mut mints = HashMap::new();
		let mut spends = HashMap::new();
		for er in evaluate_response {
			match er.validator.purpose.as_str() {
				"mint" => {
					mints.insert(
						tx.body()
							.mint()
							.expect(
								"tx.body.mint() should not be empty if we received a 'mint' response from Ogmios",
							)
							.keys()
							.get(er.validator.index as usize),
						ex_units_from_response(er),
					);
				},
				"spend" => {
					spends.insert(er.validator.index, ex_units_from_response(er));
				},
				_ => {},
			}
		}

		Ok(Costs::Costs { mints, spends })
	}
}

pub(crate) fn empty_asset_name() -> AssetName {
	AssetName::new(vec![]).expect("Hardcoded empty asset name is valid")
}

fn zero_ex_units() -> ExUnits {
	ExUnits::new(&BigNum::zero(), &BigNum::zero())
}

pub(crate) trait OgmiosUtxoExt {
	fn to_csl_tx_input(&self) -> TransactionInput;
	fn to_csl_tx_output(&self) -> Result<TransactionOutput, JsError>;
	fn to_csl(&self) -> Result<TransactionUnspentOutput, JsError>;

	fn get_asset_amount(&self, asset: &AssetId) -> u64;

	fn get_plutus_data(&self) -> Option<PlutusData>;
}

impl OgmiosUtxoExt for OgmiosUtxo {
	fn to_csl_tx_input(&self) -> TransactionInput {
		TransactionInput::new(&TransactionHash::from(self.transaction.id), self.index.into())
	}

	fn to_csl_tx_output(&self) -> Result<TransactionOutput, JsError> {
		let mut tx_out = TransactionOutput::new(
			&Address::from_bech32(&self.address).map_err(|e| {
				JsError::from_str(&format!("Couldn't convert address from ogmios: '{}'", e))
			})?,
			&self.value.to_csl()?,
		);
		if let Some(script) = self.script.clone() {
			let plutus_script_ref_opt =
				script.clone().try_into().ok().map(|plutus_script: PlutusScript| {
					ScriptRef::new_plutus_script(&plutus_script.to_csl())
				});
			let script_ref_opt = plutus_script_ref_opt.or_else(|| {
				NativeScript::from_bytes(script.cbor)
					.ok()
					.map(|native_script| ScriptRef::new_native_script(&native_script))
			});
			if let Some(script_ref) = script_ref_opt {
				tx_out.set_script_ref(&script_ref);
			}
		}
		if let Some(data) = self.get_plutus_data() {
			tx_out.set_plutus_data(&data);
		}
		Ok(tx_out)
	}

	fn to_csl(&self) -> Result<TransactionUnspentOutput, JsError> {
		Ok(TransactionUnspentOutput::new(&self.to_csl_tx_input(), &self.to_csl_tx_output()?))
	}

	fn get_asset_amount(&self, asset_id: &AssetId) -> u64 {
		self.value
			.native_tokens
			.get(&asset_id.policy_id.0)
			.cloned()
			.unwrap_or_default()
			.iter()
			.find(|asset| asset.name == asset_id.asset_name.0.to_vec())
			.map_or_else(|| 0, |asset| asset.amount)
	}

	fn get_plutus_data(&self) -> Option<PlutusData> {
		(self.datum.as_ref())
			.map(|datum| datum.bytes.clone())
			.and_then(|bytes| PlutusData::from_bytes(bytes).ok())
	}
}

/// Extension trait for [UtxoId].
pub trait UtxoIdExt {
	/// Converts domain [UtxoId] to CSL [TransactionInput].
	fn to_csl(&self) -> TransactionInput;
}

impl UtxoIdExt for UtxoId {
	fn to_csl(&self) -> TransactionInput {
		TransactionInput::new(&TransactionHash::from(self.tx_hash.0), self.index.0.into())
	}
}

#[derive(Clone)]
pub(crate) struct TransactionContext {
	/// This key is added as required signer and used to sign the transaction.
	pub(crate) payment_key: CardanoPaymentSigningKey,
	/// Used to pay for the transaction fees and uncovered transaction inputs
	/// and as source of collateral inputs
	pub(crate) payment_key_utxos: Vec<OgmiosUtxo>,
	pub(crate) network: NetworkIdKind,
	pub(crate) protocol_parameters: ProtocolParametersResponse,
	pub(crate) change_address: Address,
}

impl TransactionContext {
	/// Gets `TransactionContext`, having UTXOs for the given payment key and the network configuration,
	/// required to perform most of the partner-chains smart contract operations.
	pub(crate) async fn for_payment_key<C: QueryLedgerState + QueryNetwork>(
		payment_key: &CardanoPaymentSigningKey,
		client: &C,
	) -> Result<TransactionContext, anyhow::Error> {
		let payment_key = payment_key.clone();
		let network = client.shelley_genesis_configuration().await?.network.to_csl();
		let protocol_parameters = client.query_protocol_parameters().await?;
		let payment_address = key_hash_address(&payment_key.0.to_public().hash(), network);
		let payment_key_utxos = client.query_utxos(&[payment_address.to_bech32(None)?]).await?;
		Ok(TransactionContext {
			payment_key,
			payment_key_utxos,
			network,
			protocol_parameters,
			change_address: payment_address,
		})
	}

	pub(crate) fn with_change_address(&self, change_address: &Address) -> Self {
		Self {
			payment_key: self.payment_key.clone(),
			payment_key_utxos: self.payment_key_utxos.clone(),
			network: self.network,
			protocol_parameters: self.protocol_parameters.clone(),
			change_address: change_address.clone(),
		}
	}

	pub(crate) fn payment_key_hash(&self) -> Ed25519KeyHash {
		self.payment_key.0.to_public().hash()
	}

	pub(crate) fn sign(&self, tx: &Transaction) -> Transaction {
		let tx_hash: [u8; 32] = sidechain_domain::crypto::blake2b(tx.body().to_bytes().as_ref());
		let signature = self.payment_key.0.sign(&tx_hash);
		let mut witness_set = tx.witness_set();
		let mut vkeywitnesses = witness_set.vkeys().unwrap_or_else(Vkeywitnesses::new);
		vkeywitnesses
			.add(&Vkeywitness::new(&Vkey::new(&self.payment_key.0.to_public()), &signature));
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
		asset_name: &AssetName,
		redeemer_data: &PlutusData,
		ex_units: &ExUnits,
	) -> Result<(), JsError>;

	fn add_mint_script_tokens(
		&mut self,
		script: &PlutusScript,
		asset_name: &AssetName,
		redeemer_data: &PlutusData,
		ex_units: &ExUnits,
		amount: &Int,
	) -> Result<(), JsError>;

	/// Adds minting of tokens (with empty asset name) for the given script using reference input.
	/// IMPORTANT: Because CSL doesn't properly calculate transaction fee if the script is Native,
	/// this function adds reference input and regular native script, that is added to witnesses.
	/// This native script has to be removed from witnesses, otherwise the transaction is rejected!
	fn add_mint_script_token_using_reference_script(
		&mut self,
		script: &Script,
		ref_input: &TransactionInput,
		amount: &Int,
		costs: &Costs,
	) -> Result<(), JsError>;

	/// Adds minting of 1 token (with empty asset name) for the given script using reference input.
	/// IMPORTANT: Because CSL doesn't properly calculate transaction fee if the script is Native,
	/// this function adds reference input and regular native script, that is added to witnesses.
	/// This native script has to be removed from witnesses, otherwise the transaction is rejected!
	fn add_mint_one_script_token_using_reference_script(
		&mut self,
		script: &Script,
		ref_input: &TransactionInput,
		costs: &Costs,
	) -> Result<(), JsError> {
		self.add_mint_script_token_using_reference_script(
			script,
			ref_input,
			&Int::new_i32(1),
			costs,
		)
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
		let ma = MultiAsset::new().with_asset_amount(&policy.empty_name_asset(), 1u64)?;
		let output = amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()?;
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
		asset_name: &AssetName,
		redeemer_data: &PlutusData,
		ex_units: &ExUnits,
	) -> Result<(), JsError> {
		let mut mint_builder = self.get_mint_builder().unwrap_or(MintBuilder::new());

		let validator_source = PlutusScriptSource::new(&script.to_csl());
		let mint_witness = MintWitness::new_plutus_script(
			&validator_source,
			&Redeemer::new(&RedeemerTag::new_mint(), &0u32.into(), redeemer_data, ex_units),
		);
		mint_builder.add_asset(&mint_witness, asset_name, &Int::new_i32(1))?;
		self.set_mint_builder(&mint_builder);
		Ok(())
	}

	fn add_mint_script_tokens(
		&mut self,
		script: &PlutusScript,
		asset_name: &AssetName,
		redeemer_data: &PlutusData,
		ex_units: &ExUnits,
		amount: &Int,
	) -> Result<(), JsError> {
		let mut mint_builder = self.get_mint_builder().unwrap_or(MintBuilder::new());

		let validator_source = PlutusScriptSource::new(&script.to_csl());
		let mint_witness = MintWitness::new_plutus_script(
			&validator_source,
			&Redeemer::new(&RedeemerTag::new_mint(), &0u32.into(), redeemer_data, ex_units),
		);
		mint_builder.add_asset(&mint_witness, asset_name, amount)?;
		self.set_mint_builder(&mint_builder);
		Ok(())
	}

	fn add_mint_script_token_using_reference_script(
		&mut self,
		script: &Script,
		ref_input: &TransactionInput,
		amount: &Int,
		costs: &Costs,
	) -> Result<(), JsError> {
		let mut mint_builder = self.get_mint_builder().unwrap_or(MintBuilder::new());

		match script {
			Script::Plutus(script) => {
				let source = PlutusScriptSource::new_ref_input(
					&script.csl_script_hash(),
					ref_input,
					&script.language,
					script.bytes.len(),
				);
				let mint_witness = MintWitness::new_plutus_script(
					&source,
					&Redeemer::new(
						&RedeemerTag::new_mint(),
						&0u32.into(),
						&unit_plutus_data(),
						&costs.get_mint(script),
					),
				);
				mint_builder.add_asset(&mint_witness, &empty_asset_name(), amount)?;
				self.set_mint_builder(&mint_builder);
			},
			Script::Native(script) => {
				// new_ref_input causes invalid fee
				let source = NativeScriptSource::new(script);
				let mint_witness = MintWitness::new_native_script(&source);
				mint_builder.add_asset(&mint_witness, &empty_asset_name(), amount)?;
				self.set_mint_builder(&mint_builder);
				self.add_reference_input(ref_input);
			},
		}
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
					&ChangeConfig::new(&ctx.change_address),
				)?;
			} else {
				builder.add_collateral_inputs(ctx, collateral_inputs)?;
				builder.set_script_data_hash(&[0u8; 32].into());
				// Fake script script data hash is required for proper fee computation
				builder.add_inputs_from_and_change_with_collateral_return(
					&ctx.payment_key_utxos.to_csl()?,
					CoinSelectionStrategyCIP2::LargestFirstMultiAsset,
					&ChangeConfig::new(&ctx.change_address),
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

		let balanced_transaction =
		try_balance(self, &selected, ctx)
			.map_err(|e| JsError::from_str(&format!("Could not balance transaction. Usually it means that the payment key does not own UTXO set required to cover transaction outputs and fees or to provide collateral. Cause: {}", e)))?;

		debug_assert!(
			balanced_transaction.body().collateral().is_some(),
			"BUG: Balanced transaction should have collateral set."
		);
		debug_assert!(
			balanced_transaction.body().collateral_return().is_some(),
			"BUG: Balanced transaction should have collateral returned."
		);

		Ok(balanced_transaction)
	}
}

#[derive(Clone, Debug)]
/// Type representing a Cardano script.
pub enum Script {
	/// Plutus script
	Plutus(PlutusScript),
	/// Native script
	Native(NativeScript),
}

impl Script {
	#[cfg(test)]
	pub(crate) fn script_hash(&self) -> [u8; 28] {
		match self {
			Self::Plutus(script) => script.script_hash(),
			Self::Native(script) => {
				script.hash().to_bytes().try_into().expect("CSL script hash is always 28 bytes")
			},
		}
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
		Ok(self.with_coin_and_asset(&min_ada, ma))
	}
}

pub(crate) trait InputsBuilderExt: Sized {
	fn add_script_utxo_input(
		&mut self,
		utxo: &OgmiosUtxo,
		script: &PlutusScript,
		data: &PlutusData,
		ex_units: &ExUnits,
	) -> Result<(), JsError>;

	/// Adds ogmios inputs to the tx inputs builder.
	fn add_regular_inputs(&mut self, utxos: &[OgmiosUtxo]) -> Result<(), JsError>;

	fn with_regular_inputs(utxos: &[OgmiosUtxo]) -> Result<Self, JsError>;
}

impl InputsBuilderExt for TxInputsBuilder {
	fn add_script_utxo_input(
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

	fn add_regular_inputs(&mut self, utxos: &[OgmiosUtxo]) -> Result<(), JsError> {
		for utxo in utxos.iter() {
			self.add_regular_utxo(&utxo.to_csl()?)?;
		}
		Ok(())
	}

	fn with_regular_inputs(utxos: &[OgmiosUtxo]) -> Result<Self, JsError> {
		let mut tx_input_builder = Self::new();
		tx_input_builder.add_regular_inputs(utxos)?;
		Ok(tx_input_builder)
	}
}

pub(crate) trait AssetNameExt: Sized {
	fn to_csl(&self) -> Result<cardano_serialization_lib::AssetName, JsError>;
	fn from_csl(asset_name: cardano_serialization_lib::AssetName) -> Result<Self, JsError>;
}

impl AssetNameExt for sidechain_domain::AssetName {
	fn to_csl(&self) -> Result<cardano_serialization_lib::AssetName, JsError> {
		cardano_serialization_lib::AssetName::new(self.0.to_vec())
	}
	fn from_csl(asset_name: cardano_serialization_lib::AssetName) -> Result<Self, JsError> {
		let name = asset_name.name().try_into().map_err(|err| {
			JsError::from_str(&format!("Failed to cast CSL asset name to domain: {err:?}"))
		})?;
		Ok(Self(name))
	}
}

pub(crate) trait AssetIdExt {
	fn to_multi_asset(&self, amount: impl Into<BigNum>) -> Result<MultiAsset, JsError>;
}
impl AssetIdExt for AssetId {
	fn to_multi_asset(&self, amount: impl Into<BigNum>) -> Result<MultiAsset, JsError> {
		let mut ma = MultiAsset::new();
		let mut assets = Assets::new();
		assets.insert(&self.asset_name.to_csl()?, &amount.into());
		ma.insert(&self.policy_id.0.into(), &assets);
		Ok(ma)
	}
}

pub(crate) trait MultiAssetExt: Sized {
	fn from_ogmios_utxo(utxo: &OgmiosUtxo) -> Result<Self, JsError>;
	fn with_asset_amount(self, asset: &AssetId, amount: impl Into<BigNum>)
	-> Result<Self, JsError>;
}

impl MultiAssetExt for MultiAsset {
	fn from_ogmios_utxo(utxo: &OgmiosUtxo) -> Result<Self, JsError> {
		let mut ma = MultiAsset::new();
		for (policy, policy_assets) in utxo.value.native_tokens.iter() {
			let mut assets = Assets::new();
			for asset in policy_assets {
				assets.insert(
					&cardano_serialization_lib::AssetName::new(asset.name.clone())?,
					&asset.amount.into(),
				);
			}
			ma.insert(&PolicyID::from(*policy), &assets);
		}
		Ok(ma)
	}
	fn with_asset_amount(
		mut self,
		asset: &AssetId,
		amount: impl Into<BigNum>,
	) -> Result<Self, JsError> {
		let policy_id = asset.policy_id.0.into();
		let asset_name = asset.asset_name.to_csl()?;
		let amount: BigNum = amount.into();
		if amount > BigNum::zero() {
			self.set_asset(&policy_id, &asset_name, &amount);
			Ok(self)
		} else {
			// CSL doesn't have a public API to remove asset from MultiAsset, setting it to 0 isn't really helpful.
			let current_value = self.get_asset(&policy_id, &asset_name);
			if current_value > BigNum::zero() {
				let ma_to_sub = MultiAsset::new().with_asset_amount(asset, current_value)?;
				Ok(self.sub(&ma_to_sub))
			} else {
				Ok(self)
			}
		}
	}
}

pub(crate) trait TransactionExt: Sized {
	/// Removes all native scripts from transaction witness set.
	fn remove_native_script_witnesses(self) -> Self;
}

impl TransactionExt for Transaction {
	fn remove_native_script_witnesses(self) -> Self {
		let ws = self.witness_set();
		let mut new_ws = TransactionWitnessSet::new();
		if let Some(bootstraps) = ws.bootstraps() {
			new_ws.set_bootstraps(&bootstraps)
		}
		if let Some(plutus_data) = ws.plutus_data() {
			new_ws.set_plutus_data(&plutus_data);
		}
		if let Some(plutus_scripts) = ws.plutus_scripts() {
			new_ws.set_plutus_scripts(&plutus_scripts);
		}
		if let Some(redeemers) = ws.redeemers() {
			new_ws.set_redeemers(&redeemers);
		}
		if let Some(vkeys) = ws.vkeys() {
			new_ws.set_vkeys(&vkeys);
		}
		Transaction::new(&self.body(), &new_ws, self.auxiliary_data())
	}
}

/// In Plutus smart-contracts, unit value is represented as `Constr 0 []`.
/// It is used in many places where there is no particular value needed for redeemer.
pub(crate) fn unit_plutus_data() -> PlutusData {
	PlutusData::new_empty_constr_plutus_data(&BigNum::zero())
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::plutus_script::PlutusScript;
	use crate::test_values::protocol_parameters;
	use cardano_serialization_lib::{AssetName, Language, NetworkIdKind};
	use hex_literal::hex;
	use ogmios_client::types::{Asset, OgmiosValue};
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
			"addr_test1wpcsmvsxdjal5jxytvgd3hfntg9eav888mzfykukdjfcx2ce6knnp"
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
	fn ogmios_utxo_to_csl_with_plutus_script_attached() {
		let json = serde_json::json!(
		{
		   "transaction": {
			 "id": "1fd4a3df3e0bd48dd189878bc8e4d7419fea24c8669c84019609c897adc40f09"
		   },
		   "index": 0,
		   "address": "addr_test1vq0sjaaupatuvl9x6aefdsd4whlqtfku93068qzkhf3u2rqt9cnuq",
		   "value": {
			 "ada": {
			   "lovelace": 8904460
			 }
		   },
		   "script": {
			 "language": "plutus:v2",
			 "cbor": "59072301000033233223222253232335332232353232325333573466e1d20000021323232323232332212330010030023232325333573466e1d2000002132323232323232323232332323233323333323332332332222222222221233333333333300100d00c00b00a00900800700600500400300230013574202460026ae84044c00c8c8c8c94ccd5cd19b87480000084cc8848cc00400c008c070d5d080098029aba135744002260489201035054310035573c0046aae74004dd5000998018009aba100f23232325333573466e1d20000021323232333322221233330010050040030023232325333573466e1d20000021332212330010030023020357420026600803e6ae84d5d100089814a481035054310035573c0046aae74004dd51aba1004300835742006646464a666ae68cdc3a4000004224440062a666ae68cdc3a4004004264244460020086eb8d5d08008a999ab9a3370e9002001099091118010021aba100113029491035054310035573c0046aae74004dd51aba10023300175c6ae84d5d1001111919192999ab9a3370e900100108910008a999ab9a3370e9000001099091180100198029aba10011302a491035054310035573c0046aae74004dd50009aba20013574400226046921035054310035573c0046aae74004dd500098009aba100d30013574201860046004eb4cc00404cd5d080519980200a3ad35742012646464a666ae68cdc3a40000042646466442466002006004646464a666ae68cdc3a40000042664424660020060046600aeb4d5d080098021aba1357440022604c921035054310035573c0046aae74004dd51aba10033232325333573466e1d20000021332212330010030023300575a6ae84004c010d5d09aba2001130264901035054310035573c0046aae74004dd51aba1357440064646464a666ae68cdc3a400000420482a666ae68cdc3a4004004204a2604c921035054310035573c0046aae74004dd5000911919192999ab9a3370e9000001089110010a999ab9a3370e90010010990911180180218029aba100115333573466e1d20040021122200113026491035054310035573c0046aae74004dd500089810a49035054310035573c0046aae74004dd51aba10083300175c6ae8401c8c88c008dd60009813111999aab9f0012028233502730043574200460066ae88008084ccc00c044008d5d0802998008011aba1004300275c40024464460046eac004c09088cccd55cf800901311919a8131991091980080180118031aab9d001300535573c00260086ae8800cd5d080100f98099aba1357440026ae88004d5d10009aba2001357440026ae88004d5d10009aba2001357440026ae88004d5d100089808249035054310035573c0046aae74004dd51aba10073001357426ae8801c8c8c8c94ccd5cd19b87480000084c848888c00c014dd71aba100115333573466e1d20020021321222230010053008357420022a666ae68cdc3a400800426424444600400a600c6ae8400454ccd5cd19b87480180084c848888c010014c014d5d080089808249035054310035573c0046aae74004dd500091919192999ab9a3370e900000109909111111180280418029aba100115333573466e1d20020021321222222230070083005357420022a666ae68cdc3a400800426644244444446600c012010600a6ae84004dd71aba1357440022a666ae68cdc3a400c0042664424444444660040120106eb8d5d08009bae357426ae8800454ccd5cd19b87480200084cc8848888888cc004024020dd71aba1001375a6ae84d5d10008a999ab9a3370e90050010891111110020a999ab9a3370e900600108911111100189807a49035054310035573c0046aae74004dd500091919192999ab9a3370e9000001099091180100198029aba100115333573466e1d2002002132333222122333001005004003375a6ae84008dd69aba1001375a6ae84d5d10009aba20011300e4901035054310035573c0046aae74004dd500091919192999ab9a3370e900000109909118010019bae357420022a666ae68cdc3a400400426424460020066eb8d5d080089806a481035054310035573c0046aae74004dd500091919192999ab9a3370e900000109991091980080180118029aba1001375a6ae84d5d1000898062481035054310035573c0046aae74004dd500091919192999ab9a3370e900000109bae3574200226016921035054310035573c0046aae74004dd500089803a49035054310035573c0046aae74004dd5003111999a8009002919199ab9a337126602044a66a002290001109a801112999ab9a3371e004010260260022600c006600244444444444401066e0ccdc09a9a980091111111111100291001112999a80110a99a80108008b0b0b002a4181520e00e00ca006400a400a6eb401c48800848800440084c00524010350543500232633573800200424002600644a66a002290001109a8011119b800013006003122002122122330010040032323001001223300330020020014c01051a67998a9b0001"
		   }
		 });

		let ogmios_utxo: OgmiosUtxo = serde_json::from_value(json).unwrap();
		ogmios_utxo.to_csl().unwrap();
	}

	#[test]
	fn ogmios_utxo_to_csl_with_native_script_attached() {
		let json = serde_json::json!(
				{
		  "transaction": {
			"id": "57342ce4f30afa749bd78f0c093609366d997a1c4747d206ec7fd0aea9a35b55"
		  },
		  "index": 0,
		  "address": "addr_test1wplvesjjxtg8lhyy34ak2dr9l3kz8ged3hajvcvpanfx7rcwzvtc5",
		  "value": {
			"ada": {
			  "lovelace": 1430920
			},
			"ab81fe48f392989bd215f9fdc25ece3335a248696b2a64abc1acb595": {
			  "56657273696f6e206f7261636c65": 1
			}
		  },
		  "datum": "9f1820581cab81fe48f392989bd215f9fdc25ece3335a248696b2a64abc1acb595ff",
		  "script": {
			"language": "native",
			"json": {
			  "clause": "some",
			  "atLeast": 1,
			  "from": [
				{
				  "clause": "signature",
				  "from": "e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
				}
			  ]
			},
			"cbor": "830301818200581ce8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
		  }
		});

		let ogmios_utxo: OgmiosUtxo = serde_json::from_value(json).unwrap();
		ogmios_utxo.to_csl().unwrap();
	}
}

#[cfg(test)]
mod prop_tests {
	use super::{
		OgmiosUtxoExt, TransactionBuilderExt, TransactionContext, empty_asset_name,
		get_builder_config, unit_plutus_data, zero_ex_units,
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
			change_address: payment_addr(),
		};
		let mut tx_builder = TransactionBuilder::new(&get_builder_config(&ctx).unwrap());
		tx_builder
			.add_mint_one_script_token(
				&test_policy(),
				&empty_asset_name(),
				&unit_plutus_data(),
				&zero_ex_units(),
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
			payment_key_utxos: payment_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
			change_address: payment_addr(),
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
