//! Offchain actions to initialize bridge (IlliquidCirculionSupply) and make deposits.

mod create_utxos;
mod deposit;
mod init;

use crate::{
	csl::{OgmiosUtxoExt, OgmiosValueExt, TransactionContext},
	scripts_data,
	versioning_system::find_script_utxo,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	BigInt, ExUnits, JsError, PlutusData, PlutusScriptSource, PlutusWitness, Redeemer, RedeemerTag,
	TxInputsBuilder,
};
pub use create_utxos::create_validator_utxos;
pub use deposit::{deposit_with_ics_spend, deposit_without_ics_input};
pub use init::init_ics_scripts;
use ogmios_client::{query_ledger_state::QueryLedgerState, types::OgmiosUtxo};
use sidechain_domain::{AssetId, AssetName, UtxoId, crypto::blake2b};

/// Illiquid Circulation Supply smart contracts data
#[derive(Clone, Debug)]
pub(crate) struct ICSData {
	pub(crate) scripts: scripts_data::ICSScripts,
	/// Versioning System UTXO that keeps a policy of tokens used to maintain some minimal number of UTXOs at the validator.
	pub(crate) auth_policy_version_utxo: OgmiosUtxo,
	pub(crate) validator_version_utxo: OgmiosUtxo,
}

impl ICSData {
	pub(crate) async fn get<T: QueryLedgerState>(
		genesis_utxo: UtxoId,
		ctx: &TransactionContext,
		client: &T,
	) -> Result<Self, anyhow::Error> {
		let version_oracle = scripts_data::version_oracle(genesis_utxo, ctx.network)?;
		let validator_version_utxo = find_script_utxo(
			raw_scripts::ScriptId::IlliquidCirculationSupplyValidator as u32,
			&version_oracle,
			ctx,
			client,
		)
		.await?
		.ok_or_else(|| {
			anyhow!(
				"Illiquid Circulation Supply Validator Version Utxo not found, is the Bridge initialized?"
			)
		})?;
		let auth_policy_version_utxo = find_script_utxo(
			raw_scripts::ScriptId::IlliquidCirculationSupplyAuthorityTokenPolicy as u32,
			&version_oracle,
			ctx,
			client,
		)
		.await?
		.ok_or_else(|| {
			anyhow!("Illiquid Circulation Supply Authority Token Policy Version Utxo not found, is the Bridge initialized?")
		})?;
		let scripts = scripts_data::ics_scripts(genesis_utxo, ctx.network)?;
		Ok(ICSData { scripts, auth_policy_version_utxo, validator_version_utxo })
	}

	pub(crate) async fn get_validator_utxos_with_auth_token<T: QueryLedgerState>(
		&self,
		ctx: &TransactionContext,
		client: &T,
	) -> Result<Vec<OgmiosUtxo>, anyhow::Error> {
		let validator_address = self.scripts.validator.address(ctx.network).to_bech32(None)?;
		let validator_utxos = client.query_utxos(&[validator_address]).await?;

		let auth_token_asset_id = AssetId {
			policy_id: self.scripts.auth_policy.policy_id(),
			asset_name: AssetName::empty(),
		};

		Ok(validator_utxos
			.into_iter()
			.filter(|utxo| utxo.get_asset_amount(&auth_token_asset_id) == 1u64)
			.collect())
	}
}

/// Selects one from input utxos. To avoid randomness we take the one that combined with user own utxo has the lowest hash.
pub(crate) fn select_utxo_to_spend(
	utxos: &[OgmiosUtxo],
	ctx: &TransactionContext,
) -> Option<OgmiosUtxo> {
	utxos
		.into_iter()
		.map(|u| {
			let utxo_id = u.utxo_id();
			let mut v: Vec<u8> = utxo_id.tx_hash.0.to_vec();
			v.append(&mut utxo_id.index.0.to_be_bytes().to_vec());
			v.append(&mut ctx.payment_key_hash().to_bytes());
			let hash: [u8; 32] = blake2b(&v);
			(hash, u)
		})
		.min_by_key(|k| k.0)
		.map(|kv| kv.1.clone())
}

pub(crate) fn add_ics_utxo_input_with_validator_script_reference(
	inputs: &mut TxInputsBuilder,
	ics_utxo: &OgmiosUtxo,
	ics_data: &ICSData,
	cost: &ExUnits,
) -> Result<(), JsError> {
	let input = ics_utxo.to_csl_tx_input();
	let amount = ics_utxo.value.to_csl()?;
	let script = &ics_data.scripts.validator;
	let witness = PlutusWitness::new_with_ref_without_datum(
		&PlutusScriptSource::new_ref_input(
			&script.csl_script_hash(),
			&ics_data.validator_version_utxo.to_csl_tx_input(),
			&script.language,
			script.bytes.len(),
		),
		&Redeemer::new(
			&RedeemerTag::new_spend(),
			&0u32.into(),
			&PlutusData::new_integer(&BigInt::zero()),
			cost,
		),
	);
	inputs.add_plutus_script_input(&witness, &input, &amount);
	Ok(())
}
