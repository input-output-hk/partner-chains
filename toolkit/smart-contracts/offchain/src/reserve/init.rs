//! Initialization of the reserve management is execution of two similar transaction to
//! initialize two scripts: Reserve Management Validator and Reserve Management Policy
//!
//! Transaction for each of these scripts should have:
//! * an output to Version Oracle Validator address that should:
//! * * have script reference with the script being initialized attached, script should be applied with Version Oracle Policy Id
//! * * contain 1 token of Version Oracle Policy with "Version oracle" asset name, minted in this transaction
//! * * * mint redeemer should be Constr(1, [Int: SCRIPT_ID, Bytes: Applied Script Hash])
//! * * have Plutus Data that is [Int: SCRIPT_ID, Bytes: Version Oracle Policy Id]
//! * an output to the current governance (holder of governance token) that should:
//! * * contain a new Governance Policy token, minted in this transaction,
//! * * * mint redeemer should be empty constructor Plutus Data
//! * a script reference input of the current Governance UTXO
//! * signature of the current governance

use crate::{
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::TransactionContext,
	multisig::MultiSigSmartContractResult,
	plutus_script, scripts_data,
	versioning_system::{ScriptData, initialize_script},
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use raw_scripts::{RESERVE_AUTH_POLICY, RESERVE_VALIDATOR, ScriptId};
use sidechain_domain::UtxoId;

/// Stores smart contracts used for reserve management in the versioning system.
/// Scripts stored are:
///  - Reserve Management Validator
///  - Reserve Management Policy
///  - Illiquid Circulation Supply Validator
///  - Illiquid Circulation Auth Token Policy
pub async fn init_reserve_management<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<Vec<MultiSigSmartContractResult>> {
	let payment_ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let version_oracle = scripts_data::version_oracle(genesis_utxo, payment_ctx.network)?;

	let reserve_validator = ScriptData::new(
		"Reserve Management Validator",
		plutus_script![RESERVE_VALIDATOR, version_oracle.policy_id_as_plutus_data()]?
			.bytes
			.to_vec(),
		ScriptId::ReserveValidator,
	);
	let reserve_policy = ScriptData::new(
		"Reserve Management Policy",
		plutus_script![RESERVE_AUTH_POLICY, version_oracle.policy_id_as_plutus_data()]?
			.bytes
			.to_vec(),
		ScriptId::ReserveAuthPolicy,
	);

	Ok(vec![
		initialize_script(reserve_validator, genesis_utxo, payment_key, client, await_tx).await?,
		initialize_script(reserve_policy, genesis_utxo, payment_key, client, await_tx).await?,
	]
	.into_iter()
	.flatten()
	.collect())
}
