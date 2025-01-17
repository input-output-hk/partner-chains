//! All smart-contracts related to Rewards Token Reserve Management

use crate::csl::OgmiosUtxoExt;
use crate::{csl::TransactionContext, scripts_data};
use anyhow::anyhow;
use cardano_serialization_lib::PlutusData;
use init::find_script_utxo;
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::reserve::ReserveDatum;
use sidechain_domain::{AssetId, AssetName, UtxoId};

pub mod create;
pub mod deposit;
pub mod init;
pub mod update_settings;

#[derive(Clone, Debug)]
pub(crate) struct ReserveData {
	pub(crate) scripts: scripts_data::ReserveScripts,
	pub(crate) auth_policy_version_utxo: OgmiosUtxo,
	pub(crate) validator_version_utxo: OgmiosUtxo,
	pub(crate) illiquid_circulation_supply_validator_version_utxo: OgmiosUtxo,
}

#[derive(Clone, Debug)]
pub(crate) struct ReserveUtxo {
	pub(crate) reserve_utxo: OgmiosUtxo,
	pub(crate) reserve_settings: ReserveDatum,
}

impl ReserveData {
	pub(crate) async fn get<
		T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	>(
		genesis_utxo: UtxoId,
		ctx: &TransactionContext,
		client: &T,
	) -> Result<Self, anyhow::Error> {
		let version_oracle = scripts_data::version_oracle(genesis_utxo, ctx.network)?;
		let auth_policy_version_utxo =
			find_script_utxo(
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
		let illiquid_circulation_supply_validator_version_utxo = find_script_utxo(
			raw_scripts::ScriptId::IlliquidCirculationSupplyValidator as u32,
			&version_oracle,
			ctx,
			client,
		)
		.await?
		.ok_or_else(|| {
			anyhow!("Illiquid Circulation Supply Validator Version Utxo not found, is the Reserve Token Management initialized?")
		})?;
		let scripts = scripts_data::reserve_scripts(genesis_utxo, ctx.network)?;
		Ok(ReserveData {
			scripts,
			auth_policy_version_utxo,
			validator_version_utxo,
			illiquid_circulation_supply_validator_version_utxo,
		})
	}

	pub(crate) async fn get_reserve_utxo<
		T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	>(
		&self,
		ctx: &TransactionContext,
		client: &T,
	) -> Result<ReserveUtxo, anyhow::Error> {
		let validator_address = self.scripts.validator.address(ctx.network).to_bech32(None)?;
		let validator_utxos = client.query_utxos(&[validator_address]).await?;

		let auth_token_asset_id = AssetId {
			policy_id: self.scripts.auth_policy.policy_id(),
			asset_name: AssetName::from_hex_unsafe(""),
		};

		let (reserve_utxo, reserve_settings) = validator_utxos
			.into_iter()
			.find_map(|utxo| {
				if utxo.get_asset_amount(&auth_token_asset_id) != 1i128 {
					return None;
				}
				utxo.clone()
					.datum
					.and_then(|d| PlutusData::from_bytes(d.bytes).ok())
					.and_then(|d| ReserveDatum::try_from(d).ok())
					.map(|d| (utxo, d))
			})
			.ok_or_else(|| {
				anyhow!("Reserve Utxo not found, is the Reserve Token Management initialized?")
			})?;

		Ok(ReserveUtxo { reserve_utxo, reserve_settings })
	}
}
