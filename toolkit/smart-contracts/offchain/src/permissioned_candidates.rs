//!
//! Permissioned candidates are stored on chain in an UTXO at the Permissioned Candidates Validator address.
//! There should be at most one UTXO at the validator address and it should contain the permissioned candidates list.
//! This UTXO should have 1 token of the Permissioned Candidates Policy with an empty asset name.
//! The datum encodes Permissioned Candidates using VersionedGenericDatum envelope with the Permissioned Candidates stored
//! in the `datum` field of it. Field should contain list of list, where each inner list is a triple of byte strings
//! `[sidechain_public_key, aura_public_key, grandpa_publicKey]`.

use crate::await_tx::{AwaitTx, FixedDelayRetries};
use crate::csl::{
	empty_asset_name, get_builder_config, CostStore, Costs, InputsBuilderExt,
	TransactionBuilderExt, TransactionContext, TransactionExt,
};
use crate::governance::GovernanceData;
use crate::multisig::{submit_or_create_tx_to_sign, MultiSigSmartContractResult};
use crate::plutus_script::PlutusScript;
use crate::{cardano_keys::CardanoPaymentSigningKey, scripts_data};
use anyhow::anyhow;
use cardano_serialization_lib::{
	BigInt, PlutusData, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId};
use ogmios_client::query_network::QueryNetwork;
use ogmios_client::transactions::Transactions;
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::permissioned_candidates::{
	permissioned_candidates_to_plutus_data, PermissionedCandidateDatums,
};
use sidechain_domain::{PermissionedCandidateData, UtxoId};

pub trait UpsertPermissionedCandidates {
	#[allow(async_fn_in_trait)]
	async fn upsert_permissioned_candidates(
		&self,
		retries: FixedDelayRetries,
		genesis_utxo: UtxoId,
		candidates: &[PermissionedCandidateData],
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>>;
}

impl<C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId>
	UpsertPermissionedCandidates for C
{
	async fn upsert_permissioned_candidates(
		&self,
		retries: FixedDelayRetries,
		genesis_utxo: UtxoId,
		candidates: &[PermissionedCandidateData],
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
		upsert_permissioned_candidates(
			genesis_utxo,
			candidates,
			payment_signing_key,
			self,
			&retries,
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
	payment_signing_key: &CardanoPaymentSigningKey,
	ogmios_client: &C,
	await_tx: &A,
) -> anyhow::Result<Option<MultiSigSmartContractResult>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, ogmios_client).await?;
	let (validator, policy) =
		scripts_data::permissioned_candidates_scripts(genesis_utxo, ctx.network)?;
	let governance_data = GovernanceData::get(genesis_utxo, ogmios_client).await?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let validator_utxos = ogmios_client.query_utxos(&[validator_address]).await?;
	let mut candidates = candidates.to_owned();
	candidates.sort();

	let result_opt = match get_current_permissioned_candidates(validator_utxos)? {
		Some((_, current_permissioned_candidates))
			if current_permissioned_candidates == *candidates =>
		{
			log::info!("Current permissioned candidates are equal to the one to be set.");
			None
		},
		Some((current_utxo, _)) => {
			log::info!(
				"Current permissioned candidates are different to the one to be set. Preparing transaction to update."
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
					await_tx,
				)
				.await?,
			)
		},
		None => {
			log::info!(
				"There aren't any permissioned candidates. Preparing transaction to insert."
			);
			Some(
				insert_permissioned_candidates(
					&validator,
					&policy,
					&candidates,
					&governance_data,
					ctx,
					ogmios_client,
					await_tx,
				)
				.await?,
			)
		},
	};
	Ok(result_opt)
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

async fn insert_permissioned_candidates<C, A>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	candidates: &[PermissionedCandidateData],
	governance_data: &GovernanceData,
	payment_ctx: TransactionContext,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult>
where
	C: Transactions + QueryLedgerState + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
{
	submit_or_create_tx_to_sign(
		governance_data,
		payment_ctx,
		|costs, ctx| {
			mint_permissioned_candidates_token_tx(
				validator,
				policy,
				candidates,
				governance_data,
				costs,
				ctx,
			)
		},
		"Insert Permissioned Candidates",
		client,
		await_tx,
	)
	.await
}

async fn update_permissioned_candidates<C, A>(
	validator: &PlutusScript,
	policy: &PlutusScript,
	candidates: &[PermissionedCandidateData],
	current_utxo: &OgmiosUtxo,
	governance_data: &GovernanceData,
	payment_ctx: TransactionContext,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<MultiSigSmartContractResult>
where
	C: Transactions + QueryNetwork + QueryLedgerState + QueryUtxoByUtxoId,
	A: AwaitTx,
{
	submit_or_create_tx_to_sign(
		governance_data,
		payment_ctx,
		|costs, ctx| {
			update_permissioned_candidates_tx(
				validator,
				policy,
				candidates,
				current_utxo,
				governance_data,
				costs,
				ctx,
			)
		},
		"Update Permissioned Candidates",
		client,
		await_tx,
	)
	.await
}

/// Builds a transaction that mints a Permissioned Candidates token and also mint governance token
fn mint_permissioned_candidates_token_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	permissioned_candidates: &[PermissionedCandidateData],
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	// The essence of transaction: mint permissioned candidates token and set output with it, mint a governance token.
	tx_builder.add_mint_one_script_token(
		policy,
		&empty_asset_name(),
		&permissioned_candidates_policy_redeemer_data(),
		&costs.get_mint(&policy.clone()),
	)?;
	tx_builder.add_output_with_one_script_token(
		validator,
		policy,
		&permissioned_candidates_to_plutus_data(permissioned_candidates),
		ctx,
	)?;

	let gov_tx_input = governance_data.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

fn update_permissioned_candidates_tx(
	validator: &PlutusScript,
	policy: &PlutusScript,
	permissioned_candidates: &[PermissionedCandidateData],
	script_utxo: &OgmiosUtxo,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	{
		let mut inputs = TxInputsBuilder::new();
		inputs.add_script_utxo_input(
			script_utxo,
			validator,
			&permissioned_candidates_policy_redeemer_data(),
			&costs.get_one_spend(),
		)?;
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
		&governance_data.policy.script(),
		&gov_tx_input,
		&costs,
	)?;

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

fn permissioned_candidates_policy_redeemer_data() -> PlutusData {
	PlutusData::new_integer(&BigInt::zero())
}

#[cfg(test)]
mod tests {
	use super::{mint_permissioned_candidates_token_tx, update_permissioned_candidates_tx};
	use crate::{
		csl::{empty_asset_name, Costs, TransactionContext},
		governance::GovernanceData,
		test_values::*,
	};
	use cardano_serialization_lib::{Address, ExUnits, Int, NetworkIdKind, PlutusData};
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
			test_costs_mint(),
			&test_tx_context(),
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
			test_costs_update(),
			&test_tx_context(),
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

	fn test_costs_mint() -> Costs {
		Costs::new(
			vec![
				(test_policy().csl_script_hash(), permissioned_candidates_ex_units()),
				(test_governance_policy().script().script_hash().into(), governance_ex_units()),
			]
			.into_iter()
			.collect(),
			vec![(0, permissioned_candidates_ex_units())].into_iter().collect(),
		)
	}

	fn test_costs_update() -> Costs {
		Costs::new(
			vec![(test_governance_policy().script().script_hash().into(), governance_ex_units())]
				.into_iter()
				.collect(),
			vec![(0, permissioned_candidates_ex_units())].into_iter().collect(),
		)
	}

	fn test_goveranance_utxo() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [123; 32] }, index: 17, ..Default::default() }
	}

	fn test_governance_data() -> GovernanceData {
		GovernanceData { policy: test_governance_policy(), utxo: test_goveranance_utxo() }
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
			change_address: payment_addr(),
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
