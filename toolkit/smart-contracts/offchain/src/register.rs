use crate::cardano_keys::CardanoPaymentSigningKey;
use crate::csl::TransactionOutputAmountBuilderExt;
use crate::csl::{
	CostStore, Costs, InputsBuilderExt, TransactionBuilderExt, TransactionContext, unit_plutus_data,
};
use crate::{
	OffchainError,
	await_tx::{AwaitTx, FixedDelayRetries},
	plutus_script::PlutusScript,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	PlutusData, Transaction, TransactionBuilder, TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::registered_candidates::{
	RegisterValidatorDatum, candidate_registration_to_plutus_data,
};
use sidechain_domain::*;

pub trait Register {
	#[allow(async_fn_in_trait)]
	async fn register(
		&self,
		retries: FixedDelayRetries,
		genesis_utxo: UtxoId,
		candidate_registration: &CandidateRegistration,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> Result<Option<McTxHash>, OffchainError>;
}

impl<T> Register for T
where
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
{
	async fn register(
		&self,
		retries: FixedDelayRetries,
		genesis_utxo: UtxoId,
		candidate_registration: &CandidateRegistration,
		payment_signing_key: &CardanoPaymentSigningKey,
	) -> Result<Option<McTxHash>, OffchainError> {
		run_register(genesis_utxo, candidate_registration, payment_signing_key, self, retries)
			.await
			.map_err(|e| OffchainError::InternalError(e.to_string()))
	}
}

pub async fn run_register<
	C: QueryLedgerState + QueryNetwork + QueryUtxoByUtxoId + Transactions,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	candidate_registration: &CandidateRegistration,
	payment_signing_key: &CardanoPaymentSigningKey,
	client: &C,
	await_tx: A,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, client).await?;
	let validator = crate::scripts_data::registered_candidates_scripts(genesis_utxo)?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let registration_utxo = ctx
		.payment_key_utxos
		.iter()
		.find(|u| u.utxo_id() == candidate_registration.registration_utxo)
		.ok_or(anyhow!("registration utxo not found at payment address"))?;
	let all_registration_utxos = client.query_utxos(&[validator_address]).await?;
	let own_registrations = get_own_registrations(
		candidate_registration.own_pkh,
		candidate_registration.stake_ownership.pub_key.clone(),
		&all_registration_utxos,
	);

	if own_registrations.iter().any(|(_, existing_registration)| {
		candidate_registration.matches_keys(existing_registration)
	}) {
		log::info!("✅ Candidate already registered with same keys.");
		return Ok(None);
	}
	let own_registration_utxos = own_registrations.iter().map(|r| r.0.clone()).collect::<Vec<_>>();

	let tx = Costs::calculate_costs(
		|costs| {
			register_tx(
				&validator,
				candidate_registration,
				registration_utxo,
				&own_registration_utxos,
				costs,
				&ctx,
			)
		},
		client,
	)
	.await?;

	let signed_tx = ctx.sign(&tx).to_bytes();
	let result = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit candidate registration transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let tx_id = result.transaction.id;
	log::info!("✅ Transaction submitted. ID: {}", hex::encode(result.transaction.id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;

	Ok(Some(McTxHash(result.transaction.id)))
}

pub trait Deregister {
	#[allow(async_fn_in_trait)]
	async fn deregister(
		&self,
		retries: FixedDelayRetries,
		genesis_utxo: UtxoId,
		payment_signing_key: &CardanoPaymentSigningKey,
		stake_ownership_pub_key: StakePoolPublicKey,
	) -> Result<Option<McTxHash>, OffchainError>;
}

impl<T> Deregister for T
where
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
{
	async fn deregister(
		&self,
		retries: FixedDelayRetries,
		genesis_utxo: UtxoId,
		payment_signing_key: &CardanoPaymentSigningKey,
		stake_ownership_pub_key: StakePoolPublicKey,
	) -> Result<Option<McTxHash>, OffchainError> {
		run_deregister(genesis_utxo, payment_signing_key, stake_ownership_pub_key, self, retries)
			.await
			.map_err(|e| OffchainError::InternalError(e.to_string()))
	}
}

pub async fn run_deregister<
	C: QueryLedgerState + QueryNetwork + QueryUtxoByUtxoId + Transactions,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_signing_key: &CardanoPaymentSigningKey,
	stake_ownership_pub_key: StakePoolPublicKey,
	client: &C,
	await_tx: A,
) -> anyhow::Result<Option<McTxHash>> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, client).await?;
	let validator = crate::scripts_data::registered_candidates_scripts(genesis_utxo)?;
	let validator_address = validator.address_bech32(ctx.network)?;
	let all_registration_utxos = client.query_utxos(&[validator_address]).await?;
	let own_registrations = get_own_registrations(
		payment_signing_key.to_pub_key_hash(),
		stake_ownership_pub_key.clone(),
		&all_registration_utxos,
	);

	if own_registrations.is_empty() {
		log::info!("✅ Candidate is not registered.");
		return Ok(None);
	}

	let own_registration_utxos = own_registrations.iter().map(|r| r.0.clone()).collect::<Vec<_>>();

	let tx = Costs::calculate_costs(
		|costs| deregister_tx(&validator, &own_registration_utxos, costs, &ctx),
		client,
	)
	.await?;

	let signed_tx = ctx.sign(&tx).to_bytes();
	let result = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow!(
			"Submit candidate deregistration transaction request failed: {}, bytes: {}",
			e,
			hex::encode(tx.to_bytes())
		)
	})?;
	let tx_id = result.transaction.id;
	log::info!("✅ Transaction submitted. ID: {}", hex::encode(result.transaction.id));
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;

	Ok(Some(McTxHash(result.transaction.id)))
}

fn get_own_registrations(
	own_pkh: MainchainKeyHash,
	spo_pub_key: StakePoolPublicKey,
	validator_utxos: &[OgmiosUtxo],
) -> Vec<(OgmiosUtxo, CandidateRegistration)> {
	let mut own_registrations = Vec::new();
	for validator_utxo in validator_utxos {
		match get_candidate_registration(validator_utxo.clone()) {
			Ok(candidate_registration) => {
				if candidate_registration.stake_ownership.pub_key == spo_pub_key
					&& candidate_registration.own_pkh == own_pkh
				{
					own_registrations.push((validator_utxo.clone(), candidate_registration.clone()))
				}
			},
			Err(e) => log::debug!("Found invalid UTXO at validator address: {}", e),
		}
	}
	own_registrations
}

fn get_candidate_registration(validator_utxo: OgmiosUtxo) -> anyhow::Result<CandidateRegistration> {
	let datum = validator_utxo.datum.ok_or_else(|| anyhow!("UTXO does not have a datum"))?;
	let datum_plutus_data = PlutusData::from_bytes(datum.bytes)
		.map_err(|e| anyhow!("Could not decode datum of validator script: {}", e))?;
	let register_validator_datum = RegisterValidatorDatum::try_from(datum_plutus_data)
		.map_err(|e| anyhow!("Could not decode datum of validator script: {}", e))?;
	Ok(register_validator_datum.into())
}

fn register_tx(
	validator: &PlutusScript,
	candidate_registration: &CandidateRegistration,
	registration_utxo: &OgmiosUtxo,
	own_registration_utxos: &[OgmiosUtxo],
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	{
		let mut inputs = TxInputsBuilder::new();
		for own_registration_utxo in own_registration_utxos {
			inputs.add_script_utxo_input(
				own_registration_utxo,
				validator,
				&register_redeemer_data(),
				&costs.get_one_spend(),
			)?;
		}
		inputs.add_regular_inputs(&[registration_utxo.clone()])?;
		tx_builder.set_inputs(&inputs);
	}

	{
		let datum = candidate_registration_to_plutus_data(candidate_registration);
		let amount_builder = TransactionOutputBuilder::new()
			.with_address(&validator.address(ctx.network))
			.with_plutus_data(&datum)
			.next()?;
		let output = amount_builder.with_minimum_ada(ctx)?.build()?;
		tx_builder.add_output(&output)?;
	}

	Ok(tx_builder.balance_update_and_build(ctx)?)
}

fn deregister_tx(
	validator: &PlutusScript,
	own_registration_utxos: &[OgmiosUtxo],
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	{
		let mut inputs = TxInputsBuilder::new();
		for own_registration_utxo in own_registration_utxos {
			inputs.add_script_utxo_input(
				own_registration_utxo,
				validator,
				&register_redeemer_data(),
				&costs.get_one_spend(),
			)?;
		}
		tx_builder.set_inputs(&inputs);
	}

	Ok(tx_builder.balance_update_and_build(ctx)?)
}

fn register_redeemer_data() -> PlutusData {
	unit_plutus_data()
}

#[cfg(test)]
mod tests {
	use super::register_tx;
	use crate::csl::{Costs, OgmiosUtxoExt, TransactionContext};
	use crate::test_values::{self, *};
	use cardano_serialization_lib::{Address, NetworkIdKind, Transaction, TransactionInputs};
	use ogmios_client::types::OgmiosValue;
	use ogmios_client::types::{OgmiosTx, OgmiosUtxo};
	use partner_chains_plutus_data::registered_candidates::candidate_registration_to_plutus_data;
	use proptest::{
		array::uniform32,
		collection::{hash_set, vec},
		prelude::*,
	};

	use sidechain_domain::{
		AdaBasedStaking, AuraPublicKey, CandidateRegistration, GrandpaPublicKey, MainchainKeyHash,
		MainchainSignature, McTxHash, SidechainPublicKey, SidechainSignature, UtxoId, UtxoIndex,
	};

	fn sum_lovelace(utxos: &[OgmiosUtxo]) -> u64 {
		utxos.iter().map(|utxo| utxo.value.lovelace).sum()
	}

	const MIN_UTXO_LOVELACE: u64 = 1000000;
	const FIVE_ADA: u64 = 5000000;

	fn own_pkh() -> MainchainKeyHash {
		MainchainKeyHash([0; 28])
	}
	fn candidate_registration(registration_utxo: UtxoId) -> CandidateRegistration {
		CandidateRegistration {
			stake_ownership: AdaBasedStaking {
				pub_key: test_values::stake_pool_pub_key(),
				signature: MainchainSignature([0u8; 64]),
			},
			partner_chain_pub_key: SidechainPublicKey(Vec::new()),
			partner_chain_signature: SidechainSignature(Vec::new()),
			registration_utxo,
			own_pkh: own_pkh(),
			aura_pub_key: AuraPublicKey(Vec::new()),
			grandpa_pub_key: GrandpaPublicKey(Vec::new()),
		}
	}

	fn lesser_payment_utxo() -> OgmiosUtxo {
		make_utxo(1u8, 0, 1200000, &payment_addr())
	}

	fn greater_payment_utxo() -> OgmiosUtxo {
		make_utxo(4u8, 1, 1200001, &payment_addr())
	}

	fn registration_utxo() -> OgmiosUtxo {
		make_utxo(11u8, 0, 1000000, &payment_addr())
	}

	fn validator_addr() -> Address {
		Address::from_bech32("addr_test1wpha4546lvfcau5jsrwpht9h6350m3au86fev6nwmuqz9gqer2ung")
			.unwrap()
	}

	#[test]
	fn register_tx_regression_test() {
		let payment_key_utxos =
			vec![lesser_payment_utxo(), greater_payment_utxo(), registration_utxo()];
		let ctx = TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: payment_key_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
			change_address: payment_addr(),
		};
		let own_registration_utxos = vec![payment_key_utxos.get(1).unwrap().clone()];
		let registration_utxo = payment_key_utxos.first().unwrap();
		let candidate_registration = candidate_registration(registration_utxo.utxo_id());
		let tx = register_tx(
			&test_values::test_validator(),
			&candidate_registration,
			registration_utxo,
			&own_registration_utxos,
			Costs::ZeroCosts,
			&ctx,
		)
		.unwrap();

		let body = tx.body();
		let inputs = body.inputs();
		// Both inputs are used to cover transaction
		assert_eq!(
			inputs.get(0).to_string(),
			"0101010101010101010101010101010101010101010101010101010101010101#0"
		);
		assert_eq!(
			inputs.get(1).to_string(),
			"0404040404040404040404040404040404040404040404040404040404040404#1"
		);
		let outputs = body.outputs();

		let script_output = outputs.into_iter().find(|o| o.address() == validator_addr()).unwrap();
		let coins_sum = script_output.amount().coin().checked_add(&body.fee()).unwrap();
		assert_eq!(
			coins_sum,
			(greater_payment_utxo().value.lovelace + lesser_payment_utxo().value.lovelace).into()
		);
		assert_eq!(
			script_output.plutus_data().unwrap(),
			candidate_registration_to_plutus_data(&candidate_registration)
		);
	}

	fn register_transaction_balancing_test(payment_utxos: Vec<OgmiosUtxo>) {
		let payment_key_utxos = payment_utxos.clone();
		let ctx = TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: payment_key_utxos.clone(),
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
			change_address: payment_addr(),
		};
		let registration_utxo = payment_key_utxos.first().unwrap();
		let candidate_registration = candidate_registration(registration_utxo.utxo_id());
		let own_registration_utxos = if payment_utxos.len() >= 2 {
			vec![payment_utxos.get(1).unwrap().clone()]
		} else {
			Vec::new()
		};
		let tx = register_tx(
			&test_values::test_validator(),
			&candidate_registration,
			registration_utxo,
			&own_registration_utxos,
			Costs::ZeroCosts,
			&ctx,
		)
		.unwrap();

		let validator_address = &test_values::test_validator().address(ctx.network);

		used_inputs_lovelace_equals_outputs_and_fee(&tx, &payment_key_utxos.clone());
		fee_is_less_than_one_and_half_ada(&tx);
		output_at_validator_has_register_candidate_datum(
			&tx,
			&candidate_registration,
			validator_address,
		);
		spends_own_registration_utxos(&tx, &own_registration_utxos);
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

	// Exact fee depends on inputs and outputs, but it definately is less than 1.5 ADA
	fn fee_is_less_than_one_and_half_ada(tx: &Transaction) {
		assert!(tx.body().fee() <= 1500000u64.into());
	}

	fn output_at_validator_has_register_candidate_datum(
		tx: &Transaction,
		candidate_registration: &CandidateRegistration,
		validator_address: &Address,
	) {
		let outputs = tx.body().outputs();
		let validator_output =
			outputs.into_iter().find(|o| o.address() == *validator_address).unwrap();
		assert_eq!(
			validator_output.plutus_data().unwrap(),
			candidate_registration_to_plutus_data(candidate_registration)
		);
	}

	fn spends_own_registration_utxos(tx: &Transaction, own_registration_utxos: &[OgmiosUtxo]) {
		let inputs = tx.body().inputs();
		assert!(
			own_registration_utxos
				.iter()
				.all(|p| inputs.into_iter().any(|i| *i == p.to_csl_tx_input()))
		);
	}

	proptest! {
		#[test]
		fn spends_input_utxo_and_outputs_to_validator_address(payment_utxos in arb_payment_utxos(10)
			.prop_filter("Inputs total lovelace too low", |utxos| sum_lovelace(utxos) > 4000000)) {
			register_transaction_balancing_test(payment_utxos)
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
