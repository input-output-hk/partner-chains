//!
//! Specification for deposit transaction:
//!
//! Consumes:
//! - UTXO at the validator address
//! - UTXOs at payment address that have tokens to be deposited
//!
//! Produces:
//! - UTXO at the validator address with increased token amount
//! - UTXO at the payment address with change
//!
//! Reference UTOXs:
//! - Version Oracle Validator script
//! - Reserve Auth Policy script
//! - Reserve Validator script
//! - Illiquid Supply Validator script

use super::ReserveData;
use crate::{
	await_tx::AwaitTx,
	csl::{
		convert_value, empty_asset_name, get_builder_config, get_validator_budgets, zero_ex_units,
		OgmiosUtxoExt, TransactionBuilderExt, TransactionContext, UtxoIdExt,
	},
	init_governance::{get_governance_data, GovernanceData},
	reserve::get_reserve_data,
	scripts_data::ReserveScripts,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	Assets, BigInt, ConstrPlutusData, DataCost, ExUnits, JsError, Language, MinOutputAdaCalculator,
	MultiAsset, PlutusData, PlutusList, PlutusScriptSource, PlutusWitness, Redeemer, RedeemerTag,
	Transaction, TransactionBuilder, TransactionOutput, TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::{OgmiosEvaluateTransactionResponse, Transactions},
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::reserve::ReserveDatum;
use sidechain_domain::{McTxHash, TokenId, UtxoId};

pub struct TokenAmount {
	pub token: TokenId,
	pub amount: u64,
}

/// Spends current UTXO at validator address and creates a new UTXO with increased token amount
pub async fn deposit_to_reserve<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	parameters: TokenAmount,
	genesis_utxo: UtxoId,
	payment_key: [u8; 32],
	client: &T,
	_await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let _governance = get_governance_data(genesis_utxo, client).await?;
	let reserve = get_reserve_data(genesis_utxo, &ctx, client).await?;

	let utxo = get_utxo_with_tokens(&reserve.scripts, &parameters.token, &ctx, client).await?
		.ok_or_else(||anyhow!("There are not UTXOs in the Reserve Validator address that contain token Reserve Auth Policy Token. Have Reserve been created already?"))?;
	let current_amount = get_token_amount(&utxo, &parameters.token);
	let _token_amount =
		TokenAmount { token: parameters.token, amount: current_amount + parameters.amount };

	todo!("implement the rest in the next PR");
}

async fn get_utxo_with_tokens<T: QueryLedgerState>(
	reward_scripts: &ReserveScripts,
	token_id: &TokenId,
	ctx: &TransactionContext,
	client: &T,
) -> Result<Option<OgmiosUtxo>, anyhow::Error> {
	let validator_address = reward_scripts.validator.address_bech32(ctx.network)?;
	let utxos = client.query_utxos(&[validator_address.clone()]).await?;
	Ok(utxos
		.into_iter()
		.find(|utxo| {
			utxo.value.native_tokens.contains_key(&reward_scripts.auth_policy.script_hash())
				&& utxo
					.datum
					.clone()
					.and_then(|datum| {
						decode_reserve_datum(datum.bytes).filter(|reserve_datum| {
							reserve_datum.immutable_settings.token == *token_id
						})
					})
					.is_some()
		})
		.clone())
}

fn decode_reserve_datum(datum_bytes: Vec<u8>) -> Option<ReserveDatum> {
	PlutusData::from_bytes(datum_bytes)
		.ok()
		.and_then(|plutus_data| ReserveDatum::try_from(plutus_data).ok())
}

fn get_token_amount(utxo: &OgmiosUtxo, token_id: &TokenId) -> u64 {
	match token_id {
		TokenId::Ada => utxo.value.lovelace,
		TokenId::AssetId { policy_id, asset_name } => utxo
			.value
			.native_tokens
			.get(&policy_id.0)
			.and_then(|assets| assets.iter().find(|asset| asset.name == asset_name.0.to_vec()))
			.map(|asset| asset.amount)
			.unwrap_or(0) // Token can be not found if the reserve was created with the initial deposit of 0 tokens
			.try_into()
			.expect("Token amount in an UTXO always fits u64"),
	}
}
