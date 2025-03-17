//! Building and submitting of the transaction that changes the current governance authority
//!
//! The transaction:
//! 1. Burns the previous governance UTXO from the version oracle validator address
//! 2. Mints exactly 1 multi-sig policy token as authentication
//! 3. Produces a new governance UTXO at the version oracle validator address with a version oracle
//!    Plutus datum attached that contains the script ID (32) and policy hash.
use crate::csl::{Costs, TransactionExt};
use crate::governance::{GovernanceData, SimpleAtLeastN};
use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::CostStore,
	csl::{InputsBuilderExt, TransactionBuilderExt, TransactionContext},
	init_governance::transaction::version_oracle_datum_output,
	plutus_script::PlutusScript,
};
use cardano_serialization_lib::{
	Language, PlutusData, Transaction, TransactionBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosTx,
};
use sidechain_domain::{MainchainKeyHash, McTxHash, UtxoId, UtxoIndex};

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_values;

pub async fn run_update_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	new_governance_authority: MainchainKeyHash,
	payment_key: &CardanoPaymentSigningKey,
	genesis_utxo_id: UtxoId,
	client: &T,
	await_tx: A,
) -> anyhow::Result<OgmiosTx> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let governance_data = GovernanceData::get(genesis_utxo_id, client).await?;

	let tx = Costs::calculate_costs(
		|costs| {
			update_governance_tx(
				raw_scripts::VERSION_ORACLE_VALIDATOR,
				raw_scripts::VERSION_ORACLE_POLICY,
				genesis_utxo_id,
				new_governance_authority,
				&governance_data,
				costs,
				&ctx,
			)
		},
		client,
	)
	.await?;

	let signed_tx = ctx.sign(&tx);

	let response = client.submit_transaction(&signed_tx.to_bytes()).await?;
	log::info!("Submitted transaction: {}", hex::encode(response.transaction.id));

	await_tx
		.await_tx_output(
			client,
			UtxoId { tx_hash: McTxHash(response.transaction.id), index: UtxoIndex(0) },
		)
		.await?;

	Ok(response.transaction)
}

fn update_governance_tx(
	version_oracle_validator: &[u8],
	version_oracle_policy: &[u8],
	genesis_utxo: UtxoId,
	new_governance_authority: MainchainKeyHash,
	governance_data: &GovernanceData,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		SimpleAtLeastN { threshold: 1, key_hashes: vec![new_governance_authority.0] }
			.to_csl_native_script();
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
		&governance_data.policy.script(),
		&governance_data.utxo_id_as_tx_input(),
		&costs,
	)?;

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle_validator.clone(),
		version_oracle_policy.clone(),
		multi_sig_policy,
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

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}
