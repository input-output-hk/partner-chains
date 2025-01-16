//!
//! Permissioned candidates are stored on chain in an UTXO at the Permissioned Candidates Validator address.
//! There should be at most one UTXO at the validator address and it should contain the permissioned candidates list.
//! This UTXO should have 1 token of the Permissioned Candidates Policy with an empty asset name.
//! The datum encodes Permissioned Candidates using VersionedGenericDatum envelope with the Permissioned Candidates stored
//! in the `datum` field of it. Field should contain list of list, where each inner list is a triple of byte strings
//! `[sidechain_public_key, aura_public_key, grandpa_publicKey]`.

use std::collections::HashMap;

use crate::await_tx::{AwaitTx, FixedDelayRetries};
use crate::csl::{
	empty_asset_name, get_builder_config, get_validator_budgets, zero_ex_units, OgmiosUtxoExt,
	OgmiosValueExt, TransactionBuilderExt, TransactionContext,
};
use crate::init_governance::{self, GovernanceData};
use crate::plutus_script::PlutusScript;
use crate::scripts_data;
use anyhow::anyhow;
use cardano_serialization_lib::{
	BigInt, ExUnits, Int, JsError, MintBuilder, MintWitness, PlutusData, PlutusScriptSource,
	PlutusWitness, Redeemer, RedeemerTag, ScriptHash, Transaction, TransactionBuilder,
	TxInputsBuilder,
};
use ogmios_client::query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId};
use ogmios_client::query_network::QueryNetwork;
use ogmios_client::transactions::Transactions;
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::permissioned_candidates::{
	permissioned_candidates_to_plutus_data, PermissionedCandidateDatums,
};
use sidechain_domain::{McTxHash, PermissionedCandidateData, UtxoId};

pub trait UpsertPermissionedCandidates {
	#[allow(async_fn_in_trait)]
	async fn upsert_permissioned_candidates(
		&self,
		genesis_utxo: UtxoId,
		candidates: &[PermissionedCandidateData],
		payment_signing_key: [u8; 32],
	) -> anyhow::Result<Option<McTxHash>>;
}

impl<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId>
	UpsertPermissionedCandidates for C
{
	async fn upsert_permissioned_candidates(
		&self,
		genesis_utxo: UtxoId,
		candidates: &[PermissionedCandidateData],
		payment_signing_key: [u8; 32],
	) -> anyhow::Result<Option<McTxHash>> {
		upsert_permissioned_candidates(
			genesis_utxo,
			candidates,
			payment_signing_key,
			self,
			&FixedDelayRetries::two_minutes(),
		)
		.await
	}
}

pub async fn upsert_permissioned_candidates<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	candidates: &[PermissionedCandidateData],
	payment_signing_key: [u8; 32],
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let (validator, policy) =
		scripts_data::permissioned_candidates_scripts(genesis_utxo, ctx.network)?;
	let governance_data = init_governance::get_governance_data(genesis_utxo, ogmios_client).await?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[validator_address]).await?;
	let mut candidates = candidates.to_owned();
	candidates.sort();

	let tx_hash_opt = match get_current_permissioned_candidates(validator_utxos)? {
		Some((_, current_permissioned_candidates))
			if current_permissioned_candidates == *candidates =>
		{
			log::info!("Current permissioned candidates are equal to the one to be set.");
			None
		},
		Some((current_utxo, _)) => {
			log::info!(
				"Current permissioned candidates are different to the one to be set. Updating."
			);
			Some(
				update_permissioned_candidates(
					&validator,
					&policy,
					&candidates,
					&current_utxo,
					&governance_data,
					ctx,
					ogmios_client,
				)
				.await?,
			)
		},
		None => {
			log::info!("There are permissioned candidates. Inserting new ones.");
			Some(
				insert_permissioned_candidates(
					&validator,
					&policy,
					&candidates,
					&governance_data,
					ctx,
					ogmios_client,
				)
				.await?,
			)
		},
	};
	if let Some(tx_hash) = tx_hash_opt {
		await_tx.await_tx_output(ogmios_client, UtxoId::new(tx_hash.0, 0)).await?;
	}
	Ok(tx_hash_opt)
}

fn get_current_permissioned_candidates(
	validator_utxos: Vec<OgmiosUtxo>,
) -> Result<Option<(OgmiosUtxo, Vec<PermissionedCandidateData>)>, anyhow::Error> {
	if let Some(utxo) = validator_utxos.first() {
		let datum = utxo.datum.clone().ok_or_else(|| {
			anyhow!("Invalid state: an UTXO at the validator script address does not have a datum")
		})?;
		let datum_plutus_data = PlutusData::from_bytes(datum.bytes).map_err(|e| {
			anyhow!("Internal error: could not decode datum of permissioned candidates validator script: {}", e)
		})?;
		let mut permissioned_candidates: Vec<PermissionedCandidateData> =
			PermissionedCandidateDatums::try_from(datum_plutus_data)
				.map_err(|e| {
					anyhow!("Internal error: could not decode datum of permissioned candidates validator script: {}", e)
				})?
				.into();
		permissioned_candidates.sort();
		Ok(Some((utxo.clone(), permissioned_candidates)))
	} else {
		Ok(None)
	}
}

async fn insert_permissioned_candidates<C>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	candidates: &[PermissionedCandidateData],
	governance_data: &GovernanceData,
	ctx: TransactionContext,
	client: &C,
) -> anyhow::Result<McTxHash>
where
	C: Transactions + QueryLedgerState + QueryNetwork,
{
	let tx = mint_permissioned_candidates_token_tx(
		validator,
		policy,
		candidates,
		governance_data,
		&ctx,
		&zero_ex_units(),
		&zero_ex_units(),
	)?;

	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate insert permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;

	let mint_keys = tx.body().mint().expect("insert D parameter transaction has two mints").keys();
	let script_to_index: HashMap<ScriptHash, usize> =
		vec![(mint_keys.get(0), 0), (mint_keys.get(1), 1)].into_iter().collect();
	let mint_ex_units = get_validator_budgets(evaluate_response).mint_ex_units;
	let policy_idx = *script_to_index.get(&policy.csl_script_hash()).unwrap();
	let gov_policy_idx =
		*script_to_index.get(&governance_data.policy_script.csl_script_hash()).unwrap();
	let policy_ex_units = mint_ex_units
		.get(policy_idx)
		.expect("Evaluate transaction response should have entry for d_param policy");
	let gov_policy_ex_units = mint_ex_units
		.get(gov_policy_idx)
		.expect("Evaluate transaction response should have entry for governance policy");

	let tx = mint_permissioned_candidates_token_tx(
		validator,
		policy,
		candidates,
		governance_data,
		&ctx,
		policy_ex_units,
		gov_policy_ex_units,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit insert permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let tx_id = McTxHash(res.transaction.id);
	log::info!("Transaction submitted: {}", hex::encode(tx_id.0));
	Ok(tx_id)
}

async fn update_permissioned_candidates<C>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	candidates: &[PermissionedCandidateData],
	current_utxo: &OgmiosUtxo,
	governance_data: &GovernanceData,
	ctx: TransactionContext,
	client: &C,
) -> anyhow::Result<McTxHash>
where
	C: Transactions + QueryNetwork + QueryLedgerState,
{
	let tx = update_permissioned_candidates_tx(
		validator,
		policy,
		candidates,
		current_utxo,
		governance_data,
		&ctx,
		&zero_ex_units(),
		&zero_ex_units(),
	)?;
	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate update permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let spend_ex_units = get_validator_budgets(evaluate_response);
	let permissioned_candidates_spend_ex_units = spend_ex_units
		.spend_ex_units
		.first()
		.ok_or_else(|| JsError::from_str("Spend ex units for Permissioned Candidates Policy are missing in Evaluate Response"))?;
	let governance_mint_ex_units = spend_ex_units.spend_ex_units.first().ok_or_else(|| {
		JsError::from_str("Mint ex units for Governance Policy are missing in Evaluate Response")
	})?;

	let tx = update_permissioned_candidates_tx(
		validator,
		policy,
		candidates,
		current_utxo,
		governance_data,
		&ctx,
		permissioned_candidates_spend_ex_units,
		governance_mint_ex_units,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit update permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let tx_id = McTxHash(res.transaction.id);
	log::info!("Update permissioned candidates transaction submitted: {}", hex::encode(tx_id.0));
	Ok(tx_id)
}

/// Builds a transaction that mints a Permissioned Candidates token and also mint governance token
fn mint_permissioned_candidates_token_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	permissioned_candidates: &[PermissionedCandidateData],
	governance_data: &init_governance::GovernanceData,
	ctx: &TransactionContext,
	permissioned_candidates_ex_units: &ExUnits,
	governance_ex_units: &ExUnits,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// The essence of transaction: mint permissioned candidates token and set output with it, mint a governance token.
	{
		// Can use `add_mint_one_script_token` here because plutus data is different for unknown reason. There is an issue ETCM-9109 to explain it.
		let mut mint_builder = MintBuilder::new();
		let validator_source = PlutusScriptSource::new(&policy.to_csl());
		let mint_witness = MintWitness::new_plutus_script(
			&validator_source,
			&Redeemer::new(
				&RedeemerTag::new_mint(),
				&0u32.into(),
				&PlutusData::new_integer(&BigInt::zero()),
				permissioned_candidates_ex_units,
			),
		);
		mint_builder.add_asset(&mint_witness, &empty_asset_name(), &Int::new_i32(1))?;
		tx_builder.set_mint_builder(&mint_builder);
	}
	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&permissioned_candidates_to_plutus_data(permissioned_candidates),
		ctx,
	)?;

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy_script,
		&gov_tx_input,
		governance_ex_units,
	)?;

	tx_builder.add_script_reference_input(&gov_tx_input, governance_data.policy_script.bytes.len());
	tx_builder.balance_update_and_build(ctx)
}

fn update_permissioned_candidates_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	permissioned_candidates: &[PermissionedCandidateData],
	script_utxo: &OgmiosUtxo,
	governance_data: &GovernanceData,
	ctx: &TransactionContext,
	permissioned_candidates_ex_units: &ExUnits,
	governance_ex_units: &ExUnits,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	// Cannot use `add_script_utxo_input` because of different redeemer plutus data, ETCM-9109
	// Also the index isn't really always 0.
	{
		let mut inputs = TxInputsBuilder::new();
		let input = script_utxo.to_csl_tx_input();
		let amount = &script_utxo.value.to_csl()?;
		let witness = PlutusWitness::new_without_datum(
			&validator.to_csl(),
			&Redeemer::new(
				&RedeemerTag::new_spend(),
				// CSL will set redeemer index for the index of script input after sorting transaction inputs
				&0u32.into(),
				&PlutusData::new_integer(&BigInt::zero()),
				permissioned_candidates_ex_units,
			),
		);
		inputs.add_plutus_script_input(&witness, &input, &amount);
		tx_builder.set_inputs(&inputs);
	}

	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&permissioned_candidates_to_plutus_data(permissioned_candidates),
		ctx,
	)?;

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy_script,
		&gov_tx_input,
		governance_ex_units,
	)?;

	tx_builder.add_script_reference_input(&gov_tx_input, governance_data.policy_script.bytes.len());
	tx_builder.balance_update_and_build(ctx)
}

#[cfg(test)]
mod tests {
	use super::{mint_permissioned_candidates_token_tx, update_permissioned_candidates_tx};
	use crate::{
		csl::{empty_asset_name, TransactionContext},
		init_governance::GovernanceData,
		plutus_script::PlutusScript,
		test_values::*,
	};
	use cardano_serialization_lib::{Address, ExUnits, Int, Language, NetworkIdKind, PlutusData};
	use hex_literal::hex;
	use ogmios_client::types::{Asset as OgmiosAsset, OgmiosTx, OgmiosUtxo, OgmiosValue};
	use partner_chains_plutus_data::permissioned_candidates::permissioned_candidates_to_plutus_data;
	use sidechain_domain::{
		AuraPublicKey, GrandpaPublicKey, PermissionedCandidateData, SidechainPublicKey,
	};

	#[test]
	fn mint_permissioned_candiates_token_tx_regression_test() {
		let tx = mint_permissioned_candidates_token_tx(
			&test_validator(),
			&test_policy(),
			&input_candidates(),
			&test_governance_data(),
			&test_tx_context(),
			&permissioned_candidates_ex_units(),
			&governance_ex_units(),
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
		// There is 1 permissioned candidates token in the validator address output
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

		let tx = update_permissioned_candidates_tx(
			&test_validator(),
			&test_policy(),
			&input_candidates(),
			&script_utxo,
			&test_governance_data(),
			&test_tx_context(),
			&permissioned_candidates_ex_units(),
			&governance_ex_units(),
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
		// There is 1 permissioned candidates token in the validator address output
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

		// Collateral return must be set
		let collateral_return = body.collateral_return().unwrap();
		assert_eq!(collateral_return.address(), payment_addr());
		let total_collateral = body.total_collateral().unwrap();
		assert_eq!(
			collateral_return.amount().coin().checked_add(&total_collateral).unwrap(),
			greater_payment_utxo().value.lovelace.into()
		);
	}

	fn permissioned_candidates_ex_units() -> ExUnits {
		ExUnits::new(&10000u32.into(), &200u32.into())
	}
	fn governance_ex_units() -> ExUnits {
		ExUnits::new(&99999u32.into(), &999u32.into())
	}

	fn test_goveranance_policy() -> PlutusScript {
		PlutusScript { bytes: hex!("88991122").into(), language: Language::new_plutus_v2() }
	}

	fn test_goveranance_utxo() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [123; 32] }, index: 17, ..Default::default() }
	}

	fn test_governance_data() -> GovernanceData {
		GovernanceData { policy_script: test_goveranance_policy(), utxo: test_goveranance_utxo() }
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

	fn input_candidates() -> Vec<PermissionedCandidateData> {
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
		permissioned_candidates_to_plutus_data(&input_candidates())
	}
}
