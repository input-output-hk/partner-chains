//! All smart-contracts related to Rewards Token Reserve Management

use crate::{
	csl::{OgmiosUtxoExt, OgmiosValueExt, TransactionContext},
	scripts_data,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	ExUnits, JsError, PlutusScriptSource, PlutusWitness, Redeemer, RedeemerTag, TxInputsBuilder,
};
use init::find_script_utxo;
use ogmios_client::{query_ledger_state::QueryLedgerState, types::OgmiosUtxo};
use partner_chains_plutus_data::reserve::{ReserveDatum, ReserveRedeemer};
use sidechain_domain::{AssetId, AssetName, UtxoId};

pub mod create;
pub mod deposit;
pub mod handover;
pub mod init;
pub mod release;
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
	pub(crate) utxo: OgmiosUtxo,
	pub(crate) datum: ReserveDatum,
}

impl ReserveData {
	pub(crate) async fn get<T: QueryLedgerState>(
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

	pub(crate) async fn get_reserve_utxo<T: QueryLedgerState>(
		&self,
		ctx: &TransactionContext,
		client: &T,
	) -> Result<ReserveUtxo, anyhow::Error> {
		let validator_address = self.scripts.validator.address(ctx.network).to_bech32(None)?;
		let validator_utxos = client.query_utxos(&[validator_address]).await?;

		let auth_token_asset_id = AssetId {
			policy_id: self.scripts.auth_policy.policy_id(),
			asset_name: AssetName::empty(),
		};

		let (reserve_utxo, reserve_settings) = validator_utxos
			.into_iter()
			.find_map(|utxo| {
				if utxo.get_asset_amount(&auth_token_asset_id) != 1u64 {
					return None;
				}
				utxo.get_plutus_data()
					.and_then(|d| ReserveDatum::try_from(d).ok())
					.map(|d| (utxo, d))
			})
			.ok_or_else(|| {
				anyhow!("Reserve Utxo not found, is the Reserve Token Management initialized?")
			})?;

		Ok(ReserveUtxo { utxo: reserve_utxo, datum: reserve_settings })
	}
}

pub struct TokenAmount {
	pub token: AssetId,
	pub amount: u64,
}

pub(crate) fn reserve_utxo_input_with_validator_script_reference(
	reserve_utxo: &OgmiosUtxo,
	reserve: &ReserveData,
	redeemer: ReserveRedeemer,
	cost: &ExUnits,
) -> Result<TxInputsBuilder, JsError> {
	let mut inputs = TxInputsBuilder::new();
	let input = reserve_utxo.to_csl_tx_input();
	let amount = reserve_utxo.value.to_csl()?;
	let script = &reserve.scripts.validator;
	let witness = PlutusWitness::new_with_ref_without_datum(
		&PlutusScriptSource::new_ref_input(
			&script.csl_script_hash(),
			&reserve.validator_version_utxo.to_csl_tx_input(),
			&script.language,
			script.bytes.len(),
		),
		&Redeemer::new(&RedeemerTag::new_spend(), &0u32.into(), &redeemer.into(), cost),
	);
	inputs.add_plutus_script_input(&witness, &input, &amount);
	Ok(inputs)
}
