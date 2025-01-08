#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use crate::{
	await_tx::{self, AwaitTx},
	csl::{
		convert_ex_units, convert_value, empty_asset_name, InputsBuilderExt, OgmiosUtxoExt,
		TransactionBuilderExt, TransactionContext,
	},
	init_governance::{self, transaction::version_oracle_datum_output, GovernanceData},
	plutus_script::PlutusScript,
	scripts_data::{multisig_governance_policy_configuration, version_scripts_and_address},
};
use anyhow::{anyhow, Context};
use cardano_serialization_lib::{
	Coin, DatumSource, ExUnits, Int, LanguageKind, MintBuilder, MintWitness, MultiAsset,
	PlutusData, PlutusScriptSource, PlutusWitness, Redeemer, RedeemerTag, Transaction,
	TransactionBuilder, TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::{OgmiosTx, OgmiosUtxo},
};
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use sidechain_domain::{
	byte_string::ByteString, MainchainAddressHash, MainchainPrivateKey, McTxHash, UtxoId, UtxoIndex,
};

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_values;

pub async fn run_update_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	new_governance_authority: MainchainAddressHash,
	payment_key: MainchainPrivateKey,
	genesis_utxo_id: UtxoId,
	client: &T,
	await_tx: A,
) -> anyhow::Result<OgmiosTx> {
	let tx_context = TransactionContext::for_payment_key(payment_key.0, client).await?;
	let (version_validator, version_policy, version_validator_address) =
		version_scripts_and_address(genesis_utxo_id, tx_context.network)?;

	log::info!(
		"Querying version oracle validator address ({version_validator_address}) for utxos..."
	);
	let version_utxos = client.query_utxos(&[version_validator_address.clone()]).await?;

	let governance_data = init_governance::get_governance_data(genesis_utxo_id, client).await?;

	let tx = update_governance_tx(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		genesis_utxo_id,
		new_governance_authority,
		&tx_context,
		&governance_data,
		ExUnits::new(&0u64.into(), &0u64.into()),
		ExUnits::new(&0u64.into(), &0u64.into()),
	)?;

	let costs = client.evaluate_transaction(&tx.to_bytes()).await?;
	if costs.len() != 2 {
		return Err(anyhow!("Error retrieving witness costs: expected 2 entries."));
	};

	let Some(mint_cost) = costs.iter().find(|cost| cost.validator.purpose == "mint") else {
		return Err(anyhow!("Error retrieving witness costs: mint cost data missing."));
	};
	let Some(spend_cost) = costs.iter().find(|cost| cost.validator.purpose == "spend") else {
		return Err(anyhow!("Error retrieving witness costs: spend cost data missing."));
	};

	let tx = update_governance_tx(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		genesis_utxo_id,
		new_governance_authority,
		&tx_context,
		&governance_data,
		convert_ex_units(&mint_cost.budget),
		convert_ex_units(&spend_cost.budget),
	)?;
	let signed_tx = tx_context.sign(&tx);

	let response = client.submit_transaction(&signed_tx.to_bytes()).await?;
	println!("Submitted transaction: {}", hex::encode(response.transaction.id));

	await_tx
		.await_tx_output(
			client,
			UtxoId { tx_hash: McTxHash(response.transaction.id), index: UtxoIndex(0) },
		)
		.await?;

	Ok(response.transaction)
}

fn update_governance_tx(
	multi_sig_policy: &[u8],
	version_oracle_validator: &[u8],
	version_oracle_policy: &[u8],
	genesis_utxo: UtxoId,
	new_governance_authority: MainchainAddressHash,
	tx_context: &TransactionContext,
	governance_data: &GovernanceData,
	mint_ex_units: ExUnits,
	spend_ex_units: ExUnits,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		PlutusScript::from_wrapped_cbor(multi_sig_policy, LanguageKind::PlutusV2)?
			.apply_uplc_data(multisig_governance_policy_configuration(new_governance_authority))?;
	let version_oracle_validator =
		PlutusScript::from_wrapped_cbor(version_oracle_validator, LanguageKind::PlutusV2)?
			.apply_data(genesis_utxo)?;
	let version_oracle_policy =
		PlutusScript::from_wrapped_cbor(version_oracle_policy, LanguageKind::PlutusV2)?
			.apply_data(genesis_utxo)?
			.apply_uplc_data(version_oracle_validator.address_data(tx_context.network)?)?;

	let config = crate::csl::get_builder_config(tx_context)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&multi_sig_policy,
		&governance_data.utxo_id_as_tx_input(),
		&mint_ex_units,
	)?;

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle_validator.clone(),
		version_oracle_policy.clone(),
		multi_sig_policy.clone(),
		tx_context.network,
		tx_context,
	)?)?;

	tx_builder.set_inputs(&{
		let mut inputs = TxInputsBuilder::new();
		inputs.add_script_utxo_input_with_data(
			&governance_data.utxo,
			&version_oracle_validator,
			&PlutusData::new_integer(&32u32.into()),
			&spend_ex_units,
		)?;

		inputs
	});

	Ok(tx_builder.balance_update_and_build(tx_context)?)
}
