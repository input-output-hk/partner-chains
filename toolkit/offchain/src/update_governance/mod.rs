//! Building and submitting of the transaction that changes the current governance authority
//!
//! The transaction:
//! 1. Burns the previous governance UTXO from the version oracle validator address
//! 2. Mints exactly 1 multi-sig policy token as authentication
//! 3. Produces a new governance UTXO at the version oracle validator address with a version oracle
//!    Plutus datum attached that contains the script ID (32) and policy hash.
use crate::csl::Costs;
use crate::governance::GovernanceData;
use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::CostStore,
	csl::{InputsBuilderExt, TransactionBuilderExt, TransactionContext},
	init_governance::transaction::version_oracle_datum_output,
	plutus_script::PlutusScript,
	scripts_data::multisig_governance_policy_configuration,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	Language, PlutusData, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use serde_json::json;
use sidechain_domain::{
	MainchainKeyHash, McSmartContractResult,
	McSmartContractResult::{TxCBOR, TxHash},
	McTxHash, UtxoId, UtxoIndex,
};

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_values;

pub async fn run_update_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	old_governance_authority: &Vec<MainchainKeyHash>,
	new_governance_authority: &Vec<MainchainKeyHash>,
	new_governance_threshold: u8,
	payment_key: &CardanoPaymentSigningKey,
	genesis_utxo_id: UtxoId,
	client: &T,
	await_tx: A,
) -> anyhow::Result<McSmartContractResult> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance_data = GovernanceData::get(genesis_utxo_id, client).await?;

	let tx = Costs::calculate_costs(
		|costs| {
			update_governance_tx(
				raw_scripts::MULTI_SIG_POLICY,
				raw_scripts::VERSION_ORACLE_VALIDATOR,
				raw_scripts::VERSION_ORACLE_POLICY,
				genesis_utxo_id,
				old_governance_authority,
				new_governance_authority,
				new_governance_threshold,
				&governance_data,
				costs,
				&ctx,
			)
		},
		client,
	)
	.await?;

	let context_pub_key_hash =
		MainchainKeyHash(ctx.payment_key_hash().to_bytes().try_into().unwrap());

	if old_governance_authority.clone() == vec![context_pub_key_hash] {
		let signed_tx = ctx.sign(&tx).to_bytes();
		let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
			anyhow!(
				"Submit governance update transaction request failed: {}, bytes: {}",
				e,
				hex::encode(signed_tx)
			)
		})?;
		let tx_id = McTxHash(res.transaction.id);
		log::info!("Update D-parameter transaction submitted: {}", hex::encode(tx_id.0));
		await_tx
			.await_tx_output(
				client,
				UtxoId { tx_hash: McTxHash(res.transaction.id), index: UtxoIndex(0) },
			)
			.await?;
		Ok(TxHash(tx_id))
	} else {
		let tx_envelope = json!(
			{ "type": "Unwitnessed Tx ConwayEra",
			  "description": "",
			  "cborHex": hex::encode(tx.to_bytes())
			}
		);
		log::info!("Transaction envelope: {}", tx_envelope);
		Ok(TxCBOR(tx.to_bytes()))
	}
}

fn update_governance_tx(
	multi_sig_policy: &[u8],
	version_oracle_validator: &[u8],
	version_oracle_policy: &[u8],
	genesis_utxo: UtxoId,
	old_governance_authority: &Vec<MainchainKeyHash>,
	new_governance_authority: &Vec<MainchainKeyHash>,
	new_governance_threshold: u8,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		PlutusScript::from_wrapped_cbor(multi_sig_policy, Language::new_plutus_v2())?
			.apply_uplc_data(multisig_governance_policy_configuration(
				new_governance_authority,
				new_governance_threshold,
			))?;
	let version_oracle_validator =
		PlutusScript::from_wrapped_cbor(version_oracle_validator, Language::new_plutus_v2())?
			.apply_data(genesis_utxo)?;
	let version_oracle_policy =
		PlutusScript::from_wrapped_cbor(version_oracle_policy, Language::new_plutus_v2())?
			.apply_data(genesis_utxo)?
			.apply_uplc_data(version_oracle_validator.address_data(ctx.network)?)?;

	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance_data.policy_script,
		&governance_data.utxo_id_as_tx_input(),
		&costs.get_mint(&governance_data.policy_script),
	)?;

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle_validator.clone(),
		version_oracle_policy.clone(),
		multi_sig_policy.clone(),
		ctx.network,
		ctx,
	)?)?;

	tx_builder.set_inputs(&{
		let mut inputs = TxInputsBuilder::new();
		inputs.add_script_utxo_input(
			&governance_data.utxo,
			&version_oracle_validator,
			&PlutusData::new_integer(&(raw_scripts::ScriptId::GovernancePolicy as u32).into()),
			&costs.get_one_spend(),
		)?;

		inputs
	});

	for key in old_governance_authority.into_iter() {
		tx_builder.add_required_signer(&key.0.into());
	}

	Ok(tx_builder.balance_update_and_build(ctx)?)
}
