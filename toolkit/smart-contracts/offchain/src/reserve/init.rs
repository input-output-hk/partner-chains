//! Initialization of the reserve management is execution of three similar transaction to
//! initialize three scripts: Rerserve Management Validator, Reserve Management Policy and
//! Illiquid Circulation Supply Validator.
//!
//! Transaction for each of these scripts should have:
//! * an output to Version Oracle Validator address that should:
//! * * have script reference with the script being initialized attached, script should be applied with Version Oracle Policy Id
//! * * contain 1 token of Version Oracle Policy with "Version oracle" asset name, minted in this transaction
//! * * * mint redeemer should be Constr(1, [Int: SCRIPT_ID, Bytes: Applied Script Hash])
//! * * have Plutus Data that is [Int: SCRIPT_ID, Bytes: Version Oracle Policy Id]
//! * an output to the current governance (holder of governance token) that should:
//! * * contain a new Goveranance Policy token, minted in this transaction,
//! * * * mint redeemer should be empty contructor Plutus Data
//! * a script reference input of the current Goveranance UTXO
//! * signature of the current goveranance

use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		get_builder_config, CostStore, Costs, MultiAssetExt, OgmiosUtxoExt, TransactionBuilderExt,
		TransactionContext, TransactionExt, TransactionOutputAmountBuilderExt,
	},
	governance::GovernanceData,
	plutus_script::PlutusScript,
	scripts_data::{self, VersionOracleData},
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	AssetName, BigNum, ConstrPlutusData, JsError, Language, MultiAsset, PlutusData, PlutusList,
	ScriptRef, Transaction, TransactionBuilder, TransactionOutputBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use raw_scripts::{
	ScriptId, ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR, RESERVE_AUTH_POLICY, RESERVE_VALIDATOR,
};
use sidechain_domain::{McTxHash, UtxoId};

pub async fn init_reserve_management<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Vec<McTxHash>> {
	let reserve_validator = ScriptData::new(
		"Reserve Management Validator",
		RESERVE_VALIDATOR.to_vec(),
		ScriptId::ReserveValidator,
	);
	let reserve_policy = ScriptData::new(
		"Reserve Management Policy",
		RESERVE_AUTH_POLICY.to_vec(),
		ScriptId::ReserveAuthPolicy,
	);
	let ics_validator = ScriptData::new(
		"Illiquid Circulation Validator",
		ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR.to_vec(),
		ScriptId::IlliquidCirculationSupplyValidator,
	);
	Ok(vec![
		initialize_script(reserve_validator, genesis_utxo, payment_key, client, await_tx).await?,
		initialize_script(reserve_policy, genesis_utxo, payment_key, client, await_tx).await?,
		initialize_script(ics_validator, genesis_utxo, payment_key, client, await_tx).await?,
	]
	.into_iter()
	.flatten()
	.collect())
}

struct ScriptData {
	name: String,
	plutus_script: PlutusScript,
	id: u32,
}

impl ScriptData {
	fn new(name: &str, raw_bytes: Vec<u8>, id: ScriptId) -> Self {
		let plutus_script = PlutusScript::from_wrapped_cbor(&raw_bytes, Language::new_plutus_v2())
			.expect("Plutus script should be valid");
		Self { name: name.to_string(), plutus_script, id: id as u32 }
	}

	fn applied_plutus_script(
		&self,
		version_oracle: &VersionOracleData,
	) -> Result<PlutusScript, JsError> {
		self.plutus_script
			.clone()
			.apply_uplc_data(version_oracle.policy_id_as_plutus_data())
			.map_err(|e| JsError::from_str(&e.to_string()))
	}
}

async fn initialize_script<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	script: ScriptData,
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance = GovernanceData::get(genesis_utxo, client).await?;
	let version_oracle = scripts_data::version_oracle(genesis_utxo, ctx.network)?;

	if script_is_initialized(&script, &version_oracle, &ctx, client).await? {
		log::info!("Script '{}' is already initialized", script.name);
		return Ok(None);
	}

	let tx = Costs::calculate_costs(
		|costs| init_script_tx(&script, &governance, &version_oracle, costs, &ctx),
		client,
	)
	.await?;

	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Initialize Versioned '{}' transaction request failed: {}, tx bytes: {}",
			script.name,
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!(
		"Initialize Versioned '{}' transaction submitted: {}",
		script.name,
		hex::encode(tx_id)
	);
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;
	Ok(Some(McTxHash(tx_id)))
}

fn init_script_tx(
	script: &ScriptData,
	governance: &GovernanceData,
	version_oracle: &VersionOracleData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	let applied_script = script.applied_plutus_script(version_oracle)?;
	{
		let witness = PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(
			&BigNum::one(),
			&version_oracle_plutus_list(script.id, &applied_script.script_hash()),
		));
		tx_builder.add_mint_one_script_token(
			&version_oracle.policy,
			&version_oracle_asset_name(),
			&witness,
			&costs.get_mint(&version_oracle.policy.clone().into()),
		)?;
	}
	{
		let script_ref = ScriptRef::new_plutus_script(&applied_script.to_csl());
		let amount_builder = TransactionOutputBuilder::new()
			.with_address(&version_oracle.validator.address(ctx.network))
			.with_plutus_data(&PlutusData::new_list(&version_oracle_plutus_list(
				script.id,
				&version_oracle.policy.script_hash(),
			)))
			.with_script_ref(&script_ref)
			.next()?;
		let ma = MultiAsset::new()
			.with_asset_amount(&version_oracle.policy.asset(version_oracle_asset_name())?, 1u64)?;
		let output = amount_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()?;
		tx_builder.add_output(&output)?;
	}
	// Mint governance token
	let gov_tx_input = governance.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

fn version_oracle_asset_name() -> AssetName {
	AssetName::new(b"Version oracle".to_vec()).unwrap()
}

fn version_oracle_plutus_list(script_id: u32, script_hash: &[u8]) -> PlutusList {
	let mut list = PlutusList::new();
	list.add(&PlutusData::new_integer(&script_id.into()));
	list.add(&PlutusData::new_bytes(script_hash.to_vec()));
	list
}

// There exist UTXO at Version Oracle Validator with Datum that contains
// * script id of the script being initialized
// * Version Oracle Policy Id
async fn script_is_initialized<T: QueryLedgerState>(
	script: &ScriptData,
	version_oracle: &VersionOracleData,
	ctx: &TransactionContext,
	client: &T,
) -> Result<bool, anyhow::Error> {
	Ok(find_script_utxo(script.id, version_oracle, ctx, client).await?.is_some())
}

// Finds an UTXO at Version Oracle Validator with Datum that contains
// * given script id
// * Version Oracle Policy Id
pub(crate) async fn find_script_utxo<T: QueryLedgerState>(
	script_id: u32,
	version_oracle: &VersionOracleData,
	ctx: &TransactionContext,
	client: &T,
) -> Result<Option<OgmiosUtxo>, anyhow::Error> {
	let validator_address = version_oracle.validator.address(ctx.network).to_bech32(None)?;
	let validator_utxos = client.query_utxos(&[validator_address]).await?;
	// Decode datum from utxos and check if it contains script id
	Ok(validator_utxos.into_iter().find(|utxo| {
		utxo.get_plutus_data()
			.and_then(|data| TryInto::<VersionOracleDatum>::try_into(data).ok())
			.is_some_and(|datum| {
				datum.version_oracle == script_id
					&& datum.currency_symbol == version_oracle.policy.script_hash()
			})
	}))
}

#[cfg(test)]
mod tests {
	use super::{init_script_tx, ScriptData};
	use crate::{
		csl::{unit_plutus_data, Costs, OgmiosUtxoExt, TransactionContext},
		governance::GovernanceData,
		scripts_data::{self, VersionOracleData},
		test_values::{
			make_utxo, payment_addr, payment_key, protocol_parameters, test_governance_policy,
		},
	};
	use cardano_serialization_lib::{
		AssetName, BigNum, ConstrPlutusData, ExUnits, Int, NetworkIdKind, PlutusData, PlutusList,
		RedeemerTag, ScriptHash, Transaction,
	};
	use ogmios_client::types::{OgmiosTx, OgmiosUtxo};
	use raw_scripts::ScriptId;
	use sidechain_domain::UtxoId;

	#[test]
	fn init_script_tx_version_oracle_output_test() {
		let outputs = make_init_script_tx().body().outputs();
		let voo = outputs
			.into_iter()
			.find(|o| o.address() == version_oracle_validator_address())
			.expect("Init Script Transaction should have output to Version Oracle Validator");
		let voo_script_ref = voo
			.script_ref()
			.expect("Version Oracle Validator output should have script reference");
		let voo_plutus_script = voo_script_ref
			.plutus_script()
			.expect("Script reference should be Plutus script");
		assert_eq!(
			voo_plutus_script,
			test_initialized_script()
				.applied_plutus_script(&version_oracle_data())
				.unwrap()
				.to_csl()
		);
		let amount = voo
			.amount()
			.multiasset()
			.and_then(|ma| ma.get(&version_oracle_policy_csl_script_hash()))
			.and_then(|vo_ma| vo_ma.get(&AssetName::new(b"Version oracle".to_vec()).unwrap()))
			.expect("Version Oracle Validator output has a token of Version Oracle Policy");

		assert_eq!(amount, 1u64.into());

		let voo_plutus_data = voo
			.plutus_data()
			.and_then(|pd| pd.as_list())
			.expect("Version Oracle Validator output should have Plutus Data of List type");
		assert_eq!(
			voo_plutus_data.get(0),
			PlutusData::new_integer(&test_initialized_script().id.into())
		);
		assert_eq!(
			voo_plutus_data.get(1),
			PlutusData::new_bytes(version_oracle_data().policy.script_hash().to_vec())
		);
	}

	#[test]
	fn init_script_tx_change_output_test() {
		let outputs = make_init_script_tx().body().outputs();
		let change_output = outputs
			.into_iter()
			.find(|o| o.address() == payment_addr())
			.expect("Change output has to be present to keep governance token")
			.clone();
		let gov_token_amount = change_output
			.amount()
			.multiasset()
			.and_then(|ma| ma.get(&test_governance_data().policy.script().script_hash().into()))
			.and_then(|gov_ma| gov_ma.get(&AssetName::new(vec![]).unwrap()))
			.unwrap();
		assert_eq!(gov_token_amount, BigNum::one(), "Change contains one goverenance token");
	}

	#[test]
	fn init_script_tx_reference_input() {
		// a script reference input of the current Goveranance UTXO
		let ref_input = make_init_script_tx()
			.body()
			.reference_inputs()
			.expect("Init transaction should have reference input")
			.get(0);
		assert_eq!(ref_input, test_governance_input().to_csl_tx_input());
	}

	#[test]
	fn init_script_tx_mints() {
		let tx = make_init_script_tx();
		let vo_mint = tx
			.body()
			.mint()
			.and_then(|mint| mint.get(&version_oracle_policy_csl_script_hash()))
			.and_then(|assets| assets.get(0))
			.and_then(|assets| assets.get(&AssetName::new(b"Version oracle".to_vec()).unwrap()))
			.expect("Transaction should have a mint of Version Oracle Policy token");
		assert_eq!(vo_mint, Int::new_i32(1));

		let gov_mint = tx
			.body()
			.mint()
			.and_then(|mint| mint.get(&test_governance_data().policy.script().script_hash().into()))
			.and_then(|assets| assets.get(0))
			.and_then(|assets| assets.get(&AssetName::new(vec![]).unwrap()))
			.expect("Transaction should have a mint of Governance Policy token");
		assert_eq!(gov_mint, Int::new_i32(1));
	}

	#[test]
	fn init_script_tx_redeemers() {
		// This is so cumbersome, because of the CSL interface for Redeemers
		let ws = make_init_script_tx()
			.witness_set()
			.redeemers()
			.expect("Transaction has two redeemers");
		let redeemers = vec![ws.get(0), ws.get(1)];

		let expected_vo_redeemer_data = {
			let mut list = PlutusList::new();
			let script_id: u64 = test_initialized_script().id.into();
			list.add(&PlutusData::new_integer(&script_id.into()));
			list.add(&PlutusData::new_bytes(
				test_initialized_script()
					.applied_plutus_script(&version_oracle_data())
					.unwrap()
					.script_hash()
					.to_vec(),
			));
			let constr_plutus_data = ConstrPlutusData::new(&BigNum::one(), &list);
			PlutusData::new_constr_plutus_data(&constr_plutus_data)
		};

		let _ = redeemers
			.iter()
			.find(|r| {
				r.tag() == RedeemerTag::new_mint()
					&& r.data() == expected_vo_redeemer_data
					&& r.ex_units() == versioning_script_cost()
			})
			.expect("Transaction should have a mint redeemer for Version Oracle Policy");
		let _ = redeemers
			.iter()
			.find(|r| {
				r.tag() == RedeemerTag::new_mint()
					&& r.data() == unit_plutus_data()
					&& r.ex_units() == governance_script_cost()
			})
			.expect("Transaction should have a mint redeemer for Governance Policy");
	}

	fn make_init_script_tx() -> Transaction {
		init_script_tx(
			&test_initialized_script(),
			&test_governance_data(),
			&version_oracle_data(),
			test_costs(),
			&test_transaction_context(),
		)
		.unwrap()
	}

	fn test_initialized_script() -> ScriptData {
		ScriptData::new(
			"Test Script",
			raw_scripts::RESERVE_VALIDATOR.to_vec(),
			ScriptId::ReserveValidator,
		)
	}

	fn test_governance_input() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [16; 32] }, index: 0, ..Default::default() }
	}

	fn test_governance_data() -> GovernanceData {
		GovernanceData { policy: test_governance_policy(), utxo: test_governance_input() }
	}

	fn version_oracle_data() -> VersionOracleData {
		scripts_data::version_oracle(UtxoId::new([255u8; 32], 0), NetworkIdKind::Testnet).unwrap()
	}

	fn version_oracle_validator_address() -> cardano_serialization_lib::Address {
		version_oracle_data().validator.address(NetworkIdKind::Testnet)
	}

	fn version_oracle_policy_csl_script_hash() -> ScriptHash {
		version_oracle_data().policy.csl_script_hash()
	}

	fn test_transaction_context() -> TransactionContext {
		TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: vec![make_utxo(121u8, 3, 996272387, &payment_addr())],
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
			change_address: payment_addr(),
		}
	}

	fn governance_script_cost() -> ExUnits {
		ExUnits::new(&100u64.into(), &200u64.into())
	}

	fn versioning_script_cost() -> ExUnits {
		ExUnits::new(&300u64.into(), &400u64.into())
	}

	fn test_costs() -> Costs {
		Costs::new(
			vec![
				(test_governance_policy().script().script_hash().into(), governance_script_cost()),
				(version_oracle_policy_csl_script_hash(), versioning_script_cost()),
			]
			.into_iter()
			.collect(),
			std::collections::HashMap::new(),
		)
	}
}
