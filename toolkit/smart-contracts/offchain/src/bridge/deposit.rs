//! Bride transaction deposits an UTXO that contains the specified token at the IlliquidCirculationSupplyValidator.
//! Such transaction can either be simple deposit or can involve consuming exactly one UTXO from
//! the ICS Validator that has special token (for some reason called 'auth token').
//! The former increases the number of UTXOs at ICS Validator, but the latter preserve it.
//! Arbitrary number of UTXOs that does not have 'auth token' could be consumed, to decrese the number
//! of UTXOs, but it is not implemented yet.

use crate::{
	TokenAmount,
	await_tx::AwaitTx,
	cardano_keys::CardanoPaymentSigningKey,
	csl::{
		CostStore, Costs, MultiAssetExt, OgmiosUtxoExt, TransactionBuilderExt, TransactionContext,
		TransactionOutputAmountBuilderExt, get_builder_config,
	},
};
use cardano_serialization_lib::{
	Address, AssetName, BigNum, MultiAsset, PlutusData, ScriptHash, Transaction,
	TransactionBuilder, TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::bridge::TokenTransferDatumV1;
use sidechain_domain::{AssetId, McTxHash, UtxoId, byte_string::ByteString, crypto::blake2b};

use super::{ICSData, add_ics_utxo_input_with_validator_script_reference};

/// This function deposits bridge token to the Illiquid Circulation Supply from the payment wallet.
/// It does not consume existing UTXO at the validator address.
///  - `genesis_utxo`: UTxO identifying the Partner Chain.
///  - `amount` number of tokens to be deposited.
///  - `pc_address` information to partner chain node, where to transfer funds
///  - `payment_signing_key`: Signing key of the party paying fees.
///  - `await_tx`: [AwaitTx] strategy.
pub async fn deposit_without_ics_input<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	token: AssetId,
	amount: u64,
	pc_address: &[u8],
	payment_signing_key: &CardanoPaymentSigningKey,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, client).await?;
	let scripts = crate::scripts_data::get_scripts_data(genesis_utxo, ctx.network)?;
	let ics_address =
		Address::from_bech32(&scripts.addresses.illiquid_circulation_supply_validator)?;
	let tx_hash = submit_deposit_only_tx(
		&ics_address,
		TokenAmount { token, amount },
		pc_address,
		&ctx,
		client,
		await_tx,
	)
	.await?;
	Ok(tx_hash)
}

async fn submit_deposit_only_tx<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	ics_address: &Address,
	amount: TokenAmount,
	pc_address: &[u8],
	ctx: &TransactionContext,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let tx = deposit_only_tx(ics_address, amount, pc_address, ctx)?;
	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Bridge transfer transaction request failed: {}, tx bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!("Bridge transfer transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, McTxHash(tx_id)).await?;
	Ok(McTxHash(tx_id))
}

fn deposit_only_tx(
	ics_address: &Address,
	token_amount: TokenAmount,
	pc_address: &[u8],
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	let output_builder = TransactionOutputBuilder::new()
		.with_address(ics_address)
		.with_plutus_data(&to_user_transfer_datum(pc_address))
		.next()?;
	let ma = MultiAsset::new().with_asset_amount(&token_amount.token, token_amount.amount)?;
	let output = output_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()?;
	tx_builder.add_output(&output)?;
	Ok(tx_builder.balance_update_and_build(ctx)?)
}

/// This function deposits bridge token to the Illiquid Circulation Supply from the payment wallet.
/// It does spend existing UTXO (containing 'auth token') at the validator address.
///  - `genesis_utxo`: UTxO identifying the Partner Chain.
///  - `amount` number of tokens to be deposited.
///  - `pc_address` information to partner chain node, where to transfer funds
///  - `payment_signing_key`: Signing key of the party paying fees.
///  - `await_tx`: [AwaitTx] strategy.
pub async fn deposit_with_ics_spend<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	genesis_utxo: UtxoId,
	token: AssetId,
	amount: u64,
	pc_address: &[u8],
	payment_signing_key: &CardanoPaymentSigningKey,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let ctx = TransactionContext::for_payment_key(payment_signing_key, client).await?;
	let scripts = crate::scripts_data::get_scripts_data(genesis_utxo, ctx.network)?;
	let ics_data = ICSData::get(genesis_utxo, &ctx, client).await?;
	let token_amount = TokenAmount { token, amount };
	let ics_address =
		Address::from_bech32(&scripts.addresses.illiquid_circulation_supply_validator)?;
	let ics_utxos = ics_data.get_validator_utxos_with_auth_token(&ctx, client).await?;
	let ics_utxo_to_spend = select_utxo_to_spend(&ics_utxos, &ctx).ok_or(anyhow::anyhow!(
		"Cannot find UTXOs with an 'auth token' at ICS Validator! Use simple deposit instead."
	))?;
	let tx_hash = submit_tx(
		&ics_address,
		&ics_utxo_to_spend,
		&ics_data,
		token_amount,
		pc_address,
		&ctx,
		client,
		await_tx,
	)
	.await?;
	Ok(tx_hash)
}

// Selects one from input utxos. To avoid randomness we take the one that combined with user own utxo has the lowest hash.
fn select_utxo_to_spend(utxos: &[OgmiosUtxo], ctx: &TransactionContext) -> Option<OgmiosUtxo> {
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

async fn submit_tx<
	C: QueryLedgerState + QueryNetwork + Transactions + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	ics_address: &Address,
	ics_utxo: &OgmiosUtxo,
	ics_data: &ICSData,
	amount: TokenAmount,
	pc_address: &[u8],
	ctx: &TransactionContext,
	client: &C,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let tx = Costs::calculate_costs(
		|costs| deposit_tx(ics_address, ics_utxo, ics_data, amount.clone(), pc_address, ctx, costs),
		client,
	)
	.await?;

	let signed_tx = ctx.sign(&tx).to_bytes();
	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Bridge transfer transaction request failed: {}, tx bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!("Bridge transfer transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, McTxHash(tx_id)).await?;
	Ok(McTxHash(tx_id))
}

fn deposit_tx(
	ics_address: &Address,
	ics_utxo: &OgmiosUtxo,
	ics_data: &ICSData,
	token_amount: TokenAmount,
	pc_address: &[u8],
	ctx: &TransactionContext,
	costs: Costs,
) -> anyhow::Result<Transaction> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);
	let output_builder = TransactionOutputBuilder::new()
		.with_address(ics_address)
		.with_plutus_data(&to_user_transfer_datum(pc_address))
		.next()?;
	let mut ma = ics_utxo
		.to_csl()
		.unwrap()
		.output()
		.amount()
		.multiasset()
		.expect("ics_utxo has at least 'auth token'");
	let policy_id = ScriptHash::from(token_amount.token.policy_id.0);
	let mut assets = ma.get(&policy_id).unwrap_or_default();
	let asset_name = AssetName::new(token_amount.token.asset_name.0.to_vec())
		.expect("asset name that comes from ogmios is valid");
	let amount = assets.get(&asset_name).unwrap_or_default();
	assets.insert(&asset_name, &amount.checked_add(&BigNum::from(token_amount.amount))?);
	let _ = ma.insert(&policy_id, &assets);
	let output = output_builder.with_minimum_ada_and_asset(&ma, ctx)?.build()?;
	tx_builder.add_output(&output)?;

	let mut inputs = TxInputsBuilder::new();
	add_ics_utxo_input_with_validator_script_reference(
		&mut inputs,
		ics_utxo,
		ics_data,
		&costs.get_one_spend(),
	)?;
	tx_builder.set_inputs(&inputs);

	tx_builder.add_script_reference_input(
		&ics_data.validator_version_utxo.to_csl_tx_input(),
		ics_data.scripts.validator.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&ics_data.auth_policy_version_utxo.to_csl_tx_input(),
		ics_data.scripts.auth_policy.bytes.len(),
	);

	Ok(tx_builder.balance_update_and_build(ctx)?)
}

fn to_user_transfer_datum(pc_address: &[u8]) -> PlutusData {
	TokenTransferDatumV1::UserTransfer { receiver: ByteString(pc_address.to_vec()) }.into()
}
