#![allow(dead_code)]
//!
//! Permissioned candidates are stored on chain in an UTXO at the Permissioned Candidates Validator address.
//! There should be at most one UTXO at the validator address and it should contain the permissioned candidates list.
//! This UTXO should have 1 token of the Permissioned Candidates Policy with an empty asset name.
//! The datum encodes Permissioned Candidates using VersionedGenericDatum envelope with the Permissioned Candidates stored
//! in the `datum` field of it. Field should contain list of list, where each inner list is a triple of byte strings
//! `[sidechain_public_key, aura_public_key, grandpa_publicKey]`.

use crate::csl::{
	get_builder_config, get_first_validator_budget, InputsBuilderExt, TransactionBuilderExt,
	TransactionContext,
};
use crate::plutus_script::PlutusScript;
use anyhow::anyhow;
use cardano_serialization_lib::{
	ExUnits, JsError, PlutusData, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::query_ledger_state::QueryLedgerState;
use ogmios_client::query_network::QueryNetwork;
use ogmios_client::transactions::Transactions;
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::permissioned_candidates::{
	permissioned_candidates_to_plutus_data, PermissionedCandidateDatums,
};
use sidechain_domain::{McTxHash, PermissionedCandidateData, UtxoId};

pub async fn upsert_permissioned_candidates<C: QueryLedgerState + QueryNetwork + Transactions>(
	genesis_utxo: UtxoId,
	candidates: &[PermissionedCandidateData],
	payment_signing_key: [u8; 32],
	ogmios_client: &C,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let (validator, policy) =
		crate::scripts_data::permissioned_candidates_scripts(genesis_utxo, ctx.network)?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[validator_address]).await?;
	let mut candidates = candidates.to_owned();
	candidates.sort();

	match get_current_permissioned_candidates(validator_utxos)? {
		Some((_, current_permissioned_candidates))
			if current_permissioned_candidates == *candidates =>
		{
			log::info!("Current permissioned candidates are equal to the one to be set.");
			Ok(None)
		},
		Some((current_utxo, _)) => {
			log::info!(
				"Current permissioned candidates are different to the one to be set. Updating."
			);
			Ok(Some(
				update_permissioned_candidates(
					&validator,
					&policy,
					&candidates,
					&current_utxo,
					ctx,
					ogmios_client,
				)
				.await?,
			))
		},
		None => {
			log::info!("There are permissioned candidates. Inserting new ones.");
			Ok(Some(
				insert_permissioned_candidates(
					&validator,
					&policy,
					&candidates,
					ctx,
					ogmios_client,
				)
				.await?,
			))
		},
	}
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
	ctx: TransactionContext,
	client: &C,
) -> anyhow::Result<McTxHash>
where
	C: Transactions,
{
	let zero_ex_units = ExUnits::new(&0u64.into(), &0u64.into());
	let tx =
		mint_permissioned_candidates_token_tx(validator, policy, candidates, &ctx, zero_ex_units)?;
	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate insert permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let mint_witness_ex_units = get_first_validator_budget(evaluate_response)?;
	let tx = mint_permissioned_candidates_token_tx(
		validator,
		policy,
		candidates,
		&ctx,
		mint_witness_ex_units,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit insert permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	log::info!("Transaction submitted: {}", hex::encode(res.transaction.id));
	Ok(McTxHash(res.transaction.id))
}

async fn update_permissioned_candidates<C>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	candidates: &[PermissionedCandidateData],
	current_utxo: &OgmiosUtxo,
	ctx: TransactionContext,
	client: &C,
) -> anyhow::Result<McTxHash>
where
	C: Transactions,
{
	let zero_ex_units = ExUnits::new(&0u64.into(), &0u64.into());
	let tx = update_permissioned_candidates_tx(
		validator,
		policy,
		candidates,
		current_utxo,
		&ctx,
		zero_ex_units,
	)?;
	let evaluate_response = client.evaluate_transaction(&tx.to_bytes()).await.map_err(|e| {
		anyhow!(
			"Evaluate update permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let spend_ex_units = get_first_validator_budget(evaluate_response)?;

	let tx = update_permissioned_candidates_tx(
		validator,
		policy,
		candidates,
		current_utxo,
		&ctx,
		spend_ex_units,
	)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit update permissioned candidates transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	log::info!(
		"Update permissioned candidates transaction submitted: {}",
		hex::encode(res.transaction.id)
	);
	Ok(McTxHash(res.transaction.id))
}

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
		ogmios_mock::MockOgmiosClient,
		permissioned_candidates::upsert_permissioned_candidates,
		scripts_data::get_scripts_data,
		test_values::*,
	};
	use cardano_serialization_lib::{
		Address, ExUnits, Int, NetworkIdKind, PlutusData, RedeemerTag,
	};
	use hex_literal::hex;
	use ogmios_client::{
		transactions::{
			OgmiosBudget, OgmiosEvaluateTransactionResponse, OgmiosValidatorIndex,
			SubmitTransactionResponse,
		},
		types::{Asset as OgmiosAsset, OgmiosTx, OgmiosUtxo, OgmiosValue},
	};
	use partner_chains_plutus_data::permissioned_candidates::permissioned_candidates_to_plutus_data;
	use sidechain_domain::{
		AuraPublicKey, GrandpaPublicKey, McTxHash, PermissionedCandidateData, SidechainPublicKey,
		UtxoId,
	};
	use std::str::FromStr;

	#[test]
	fn mint_permissioned_candiates_token_tx_regression_test() {
		// We know the expected values were obtained with the correct code
		let ex_units = ExUnits::new(&10000u32.into(), &200u32.into());

		let tx = mint_permissioned_candidates_token_tx(
			&test_validator(),
			&test_policy(),
			&input_candidates(),
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
			&input_candidates(),
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

	fn existing_candidates() -> Vec<PermissionedCandidateData> {
		// Unordered for testing purposes
		vec![
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
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(
					hex!("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
						.into(),
				),
				aura_public_key: AuraPublicKey(
					hex!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff").into(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("0000000000000000000000000000000000000000000000000000000000000000").into(),
				),
			},
			PermissionedCandidateData {
				sidechain_public_key: SidechainPublicKey(
					hex!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")
						.into(),
				),
				aura_public_key: AuraPublicKey(
					hex!("1111111111111111111111111111111111111111111111111111111111111111").into(),
				),
				grandpa_public_key: GrandpaPublicKey(
					hex!("2222222222222222222222222222222222222222222222222222222222222222").into(),
				),
			},
		]
	}

	fn expected_plutus_data() -> PlutusData {
		permissioned_candidates_to_plutus_data(&input_candidates())
	}

	#[tokio::test]
	async fn upsert_inserts_when_there_are_no_candidates_on_chain() {
		let client = mock_client(
			vec![OgmiosEvaluateTransactionResponse {
				validator: OgmiosValidatorIndex { index: 0, purpose: "mint".into() },
				budget: OgmiosBudget { memory: 519278, cpu: 155707522 },
			}],
			vec![],
		);
		let tx = upsert_permissioned_candidates(
			test_genesis_utxo(),
			&input_candidates(),
			payment_key().as_bytes().try_into().unwrap(),
			&client,
		)
		.await
		.unwrap();
		assert_eq!(tx, Some(McTxHash(test_upsert_tx_hash())))
	}

	#[tokio::test]
	async fn upsert_does_nothing_if_existing_candidates_are_equal_to_requested() {
		let mut existing_candidates_in_different_order = existing_candidates();
		existing_candidates_in_different_order.reverse();
		let client = mock_client(vec![], vec![script_utxo(&existing_candidates())]);

		let tx = upsert_permissioned_candidates(
			test_genesis_utxo(),
			&existing_candidates_in_different_order,
			payment_key().as_bytes().try_into().unwrap(),
			&client,
		)
		.await
		.unwrap();
		assert_eq!(tx, None)
	}

	#[tokio::test]
	async fn upsert_updates_candidates_when_requested_are_different_to_existing() {
		let client = mock_client(
			vec![OgmiosEvaluateTransactionResponse {
				validator: OgmiosValidatorIndex { index: 0, purpose: "spend".into() },
				budget: OgmiosBudget { memory: 519278, cpu: 155707522 },
			}],
			vec![script_utxo(&existing_candidates())],
		);
		let tx = upsert_permissioned_candidates(
			test_genesis_utxo(),
			&input_candidates(),
			payment_key().as_bytes().try_into().unwrap(),
			&client,
		)
		.await
		.unwrap();
		assert_eq!(tx, Some(McTxHash(test_upsert_tx_hash())))
	}

	// Creates an UTXO that has proper multi-asset and datum
	fn script_utxo(permissioned_candidates: &Vec<PermissionedCandidateData>) -> OgmiosUtxo {
		let plutus_data = permissioned_candidates_to_plutus_data(permissioned_candidates);
		let policy =
			crate::scripts_data::get_scripts_data(test_genesis_utxo(), NetworkIdKind::Testnet)
				.unwrap()
				.policy_ids
				.permissioned_candidates;

		OgmiosUtxo {
			transaction: OgmiosTx { id: [15; 32] },
			index: 0,
			value: OgmiosValue {
				lovelace: 10000000,
				native_tokens: vec![(policy.0, vec![OgmiosAsset { name: vec![], amount: 1 }])]
					.into_iter()
					.collect(),
			},
			address: get_scripts_data(test_genesis_utxo(), NetworkIdKind::Testnet)
				.unwrap()
				.addresses
				.permissioned_candidates_validator,
			datum: Some(ogmios_client::types::Datum { bytes: plutus_data.to_bytes() }),
			..Default::default()
		}
	}

	fn test_upsert_tx_hash() -> [u8; 32] {
		hex!("aabbaabbaabbaabbaabbaabbaabbaabbaabbaabbaabbaabbaabbaabbaabbaabb")
	}

	fn test_genesis_utxo() -> UtxoId {
		UtxoId::from_str("c389187c6cabf1cd2ca64cf8c76bf57288eb9c02ced6781935b810a1d0e7fbb4#1")
			.unwrap()
	}

	fn mock_client(
		evaluate_response: Vec<OgmiosEvaluateTransactionResponse>,
		validator_utxos: Vec<OgmiosUtxo>,
	) -> MockOgmiosClient {
		MockOgmiosClient::new()
			.with_evaluate_result(evaluate_response)
			.with_submit_result(SubmitTransactionResponse {
				transaction: test_upsert_tx_hash().into(),
			})
			.with_utxos(vec![make_utxo(1u8, 0, 15000000, &payment_addr())])
			.with_utxos(validator_utxos)
			.with_protocol_parameters(protocol_parameters())
			.with_shelley_config(shelley_config())
	}
}
