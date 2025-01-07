//! All smart-contracts related to Rewards Token Reserve Management

use crate::{csl::TransactionContext, scripts_data};
use anyhow::anyhow;
use init::find_script_utxo;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
};
use sidechain_domain::UtxoId;

pub mod create;
pub mod init;

pub(crate) struct ReserveData {
	pub(crate) scripts: scripts_data::ReserveScripts,
	pub(crate) auth_policy_version_utxo: UtxoId,
	pub(crate) validator_version_utxo: UtxoId,
}

pub(crate) async fn get_reserve_data<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
>(
	genesis_utxo: UtxoId,
	ctx: &TransactionContext,
	client: &T,
) -> Result<ReserveData, anyhow::Error> {
	let version_oracle = scripts_data::version_oracle(genesis_utxo, ctx.network)?;
	let auth_policy_version_utxo = find_script_utxo(
		raw_scripts::ScriptId::ReserveAuthPolicy as u32,
		&version_oracle,
		ctx,
		client,
	)
	.await?
	.ok_or_else(|| {
		anyhow!("Reserve Auth Version Utxo not found, is the Reserve Token Management initialized?")
	})?;
	let validator_version_utxo = find_script_utxo(
		raw_scripts::ScriptId::ReserveValidator as u32,
		&version_oracle,
		ctx,
		client,
	)
	.await?
	.ok_or_else(|| {
		anyhow!("Reserve Validator Version Utxo not found, is the Reserve Token Management initialized?")
	})?;
	let scripts = scripts_data::reserve_scripts(genesis_utxo, ctx.network)?;
	Ok(ReserveData { scripts, auth_policy_version_utxo, validator_version_utxo })
}
