#![allow(rustdoc::private_intra_doc_links)]
// This transaction releases some funds to the illiquid supply.
//! Inputs:
//!     - previous utxo at the *reserve validator*, containing the reserve tokens and the
//!       [ReserveData] plutus data with reserve configuration and release stats
//! Reference inputs:
//!     - utxo with V-Function reference script matching the hash saved in the input [ReserveData].
//!       IMPORTANT: The V-Function script will evaluate against the total number of tokens that
//!                  should have been released up to now.
//!                  The number of tokens released in a single transaction equals
//!                  `<current v-function value> - <number of previously released tokens>`.
//!     - utxo with authentication policy reference script
//!     - utxo with validator version policy reference script
//!     - utxo with illiquid supply validator reference script
//! Outputs:
//!     - utxo at the *reserve validator* containing the rest of unreleased tokens and the
//!       updated [ReserveData] plutus data
//!     - utxo at the *illiquid supply validator* containing the newly released tokens
//! Mints:
//!     - V-Function tokens in the number equal to *the total number of reserve tokens released
//!       including the ones released in this transaction*. Ie. if N tokens were already released
//!       and M tokens are being released, the transaction should mint N+M V-Function tokens.
//!       These tokens are worthless and don't serve any purpose after the transaction is done.
use super::{ReserveData, add_reserve_utxo_input_with_validator_script_reference};
use crate::{
	await_tx::AwaitTx,
	bridge::{ICSData, add_ics_utxo_input_with_validator_script_reference, select_utxo_to_spend},
	cardano_keys::CardanoPaymentSigningKey,
	csl::*,
	plutus_script::PlutusScript,
	reserve::ReserveUtxo,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	Assets, Int, MultiAsset, PlutusData, Transaction, TransactionBuilder, TransactionOutputBuilder,
	TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::*, query_network::QueryNetwork, transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::{bridge::TokenTransferDatumV1, reserve::ReserveRedeemer};
use sidechain_domain::{McTxHash, UtxoId};
use std::num::NonZero;
/// Releases funds from reserve.
pub async fn release_reserve_funds<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	amount: NonZero<u64>,
	genesis_utxo: UtxoId,
	reference_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	client: &T,
	await_tx: &A,
) -> anyhow::Result<McTxHash> {
	let ctx = TransactionContext::for_payment_key(payment_key, client).await?;
	let tip = client.get_tip().await?;
	let reserve_data = ReserveData::get(genesis_utxo, &ctx, client).await?;
	let Some(reference_utxo) = client.query_utxo_by_id(reference_utxo).await? else {
		return Err(anyhow!("Reference utxo {reference_utxo:?} not found on chain"));
	};
	let ics_data = ICSData::get(genesis_utxo, &ctx, client).await?;

	let reserve_utxo = reserve_data.get_reserve_utxo(&ctx, client).await?;
	let ics_utxos = ics_data.get_validator_utxos_with_auth_token(&ctx, client).await?;
	let ics_utxo = select_utxo_to_spend(&ics_utxos, &ctx).ok_or(anyhow::anyhow!(
		"Cannot find UTXOs with an 'auth token' at ICS Validator! Is the Bridge initialized?"
	))?;

	let tx = Costs::calculate_costs(
		|costs| {
			let tx1 = reserve_release_tx(
				&ctx,
				&reserve_data,
				&ics_data,
				&reserve_utxo,
				&ics_utxo,
				&reference_utxo,
				amount.get(),
				tip.slot,
				costs,
			)?;
			println!("tx1: {}", hex::encode(tx1.to_bytes()));
			Ok(tx1)
		},
		client,
	)
	.await?;

	let signed_tx = ctx.sign(&tx).to_bytes();

	let res = client.submit_transaction(&signed_tx).await.map_err(|e| {
		anyhow::anyhow!(
			"Reserve release transaction request failed: {}, tx bytes: {}",
			e,
			hex::encode(signed_tx)
		)
	})?;
	let tx_id = res.transaction.id;
	log::info!("Reserve release transaction submitted: {}", hex::encode(tx_id));
	await_tx.await_tx_output(client, McTxHash(tx_id)).await?;

	Ok(McTxHash(tx_id))
}

fn reserve_release_tx(
	ctx: &TransactionContext,
	reserve_data: &ReserveData,
	ics_data: &ICSData,
	previous_reserve: &ReserveUtxo,
	ics_utxo: &OgmiosUtxo,
	reference_utxo: &OgmiosUtxo,
	amount_to_transfer: u64,
	latest_slot: u64,
	costs: Costs,
) -> anyhow::Result<Transaction> {
	let token = &previous_reserve.datum.immutable_settings.token;
	let stats = &previous_reserve.datum.stats;

	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let reserve_balance = previous_reserve.utxo.get_asset_amount(token);
	let ics_balance = ics_utxo.get_asset_amount(token);
	let token_total_amount_transferred = stats.token_total_amount_transferred;
	let cumulative_total_transfer: u64 = token_total_amount_transferred
		.checked_add(amount_to_transfer)
		.expect("cumulative_total_transfer can't overflow u64");

	let left_in_reserve = reserve_balance.checked_sub(amount_to_transfer)
		.ok_or_else(||anyhow!("Not enough funds in the reserve to transfer {amount_to_transfer} tokens (reserve balance: {reserve_balance})"))?;

	println!("R RESERVE AUTH UTXO: {:?}", reserve_data.auth_policy_version_utxo);

	// Additional reference scripts
	tx_builder.add_script_reference_input(
		&reserve_data.auth_policy_version_utxo.to_csl_tx_input(),
		reserve_data.scripts.auth_policy.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&ics_data.validator_version_utxo.to_csl_tx_input(),
		ics_data.scripts.validator.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&ics_data.auth_policy_version_utxo.to_csl_tx_input(),
		reserve_data.scripts.auth_policy.bytes.len(),
	);

	// Mint v-function tokens in the number equal to the *total* number of tokens transferred.
	// This serves as a validation of the v-function value.
	let v_function = v_function_from_utxo(reference_utxo)?;
	tx_builder.add_mint_script_token_using_reference_script(
		&Script::Plutus(v_function),
		&reference_utxo.to_csl_tx_input(),
		&Int::new(&cumulative_total_transfer.into()),
		&costs,
	)?;

	let mut tx_inputs = TxInputsBuilder::new();

	let spend_indices = costs.get_spend_indices();

	let mut spend_costs = vec![
		costs.get_spend(*spend_indices.get(1).unwrap_or(&0)),
		costs.get_spend(*spend_indices.get(0).unwrap_or(&0)),
	];

	spend_costs.sort();

	// Remove tokens from the reserve
	add_reserve_utxo_input_with_validator_script_reference(
		&mut tx_inputs,
		&previous_reserve.utxo,
		reserve_data,
		ReserveRedeemer::ReleaseFromReserve,
		spend_costs.get(1).unwrap(),
	)?;

	add_ics_utxo_input_with_validator_script_reference(
		&mut tx_inputs,
		ics_utxo,
		ics_data,
		spend_costs.get(0).unwrap(),
	)?;

	tx_builder.set_inputs(&tx_inputs);

	let mut ics_tokens = ics_utxo
		.value
		.to_csl()?
		.multiasset()
		.expect("ICS UTXO should have native tokens");

	let mut assets = Assets::new();

	assets.insert(
		&token.asset_name.to_csl()?,
		&amount_to_transfer.saturating_add(ics_balance).into(),
	);

	ics_tokens.insert(&token.policy_id.0.into(), &assets);

	// Transfer released tokens to the illiquid supply
	tx_builder.add_output(&{
		TransactionOutputBuilder::new()
			.with_address(&ics_data.scripts.validator.address(ctx.network))
			.with_plutus_data(&TokenTransferDatumV1::ReserveTransfer.into())
			.next()?
			.with_minimum_ada_and_asset(&ics_tokens, ctx)?
			.build()?
	})?;

	// Return the rest of the tokens back to the reserve
	tx_builder.add_output(&{
		TransactionOutputBuilder::new()
			.with_address(&reserve_data.scripts.validator.address(ctx.network))
			.with_plutus_data(&PlutusData::from(
				previous_reserve.datum.clone().after_withdrawal(amount_to_transfer),
			))
			.next()?
			.with_minimum_ada_and_asset(
				&MultiAsset::from_ogmios_utxo(&previous_reserve.utxo)?
					.with_asset_amount(token, left_in_reserve)?,
				ctx,
			)?
			.build()?
	})?;

	tx_builder.set_validity_start_interval_bignum(latest_slot.into());

	Ok(tx_builder.balance_update_and_build(ctx)?.remove_native_script_witnesses())
}

fn v_function_from_utxo(utxo: &OgmiosUtxo) -> anyhow::Result<PlutusScript> {
	let Some(v_function_script) = utxo.script.clone() else {
		return Err(anyhow!("V-Function reference script missing from the reference UTXO",));
	};
	PlutusScript::try_from(v_function_script)
		.map_err(|val| anyhow!("{val:?} is not a valid Plutus Script"))
}

#[cfg(test)]
mod tests {
	use super::{AssetNameExt, Costs, TransactionContext, empty_asset_name, reserve_release_tx};
	use crate::plutus_script;
	use crate::{
		bridge::ICSData,
		cardano_keys::CardanoPaymentSigningKey,
		plutus_script::PlutusScript,
		reserve::{ReserveData, ReserveUtxo, release::OgmiosUtxoExt},
		scripts_data::{ICSScripts, ReserveScripts},
		test_values::{payment_addr, protocol_parameters},
	};
	use cardano_serialization_lib::{Int, NetworkIdKind, PolicyID, Transaction};
	use hex_literal::hex;
	use ogmios_client::types::{Asset, OgmiosTx, OgmiosUtxo, OgmiosValue};
	use partner_chains_plutus_data::reserve::{
		ReserveDatum, ReserveImmutableSettings, ReserveMutableSettings, ReserveStats,
	};

	use pretty_assertions::assert_eq;
	use raw_scripts::{
		EXAMPLE_V_FUNCTION_POLICY, ILLIQUID_CIRCULATION_SUPPLY_AUTHORITY_TOKEN_POLICY,
		ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR, RESERVE_AUTH_POLICY, RESERVE_VALIDATOR, RawScript,
	};
	use sidechain_domain::{AssetName, PolicyId};

	fn payment_key() -> CardanoPaymentSigningKey {
		CardanoPaymentSigningKey::from_normal_bytes(hex!(
			"94f7531c9639654b77fa7e10650702b6937e05cd868f419f54bcb8368e413f04"
		))
		.unwrap()
	}

	fn test_address_bech32() -> String {
		"addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw".into()
	}

	fn payment_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("f5e751f474e909419c714bb5666a8f810e7ed61fadad236c29f67dafc1ff398b"),
			},
			index: 1,
			value: OgmiosValue {
				lovelace: 994916563,
				native_tokens: [(
					// random native token
					hex!("08b95138e16a062fa8d623a2b1beebd59c06210f3d33690580733e73"),
					vec![Asset { name: vec![], amount: 1 }],
				)]
				.into(),
			},
			address: test_address_bech32(),

			..OgmiosUtxo::default()
		}
	}

	fn tx_context() -> TransactionContext {
		TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: vec![payment_utxo()],
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
			change_address: payment_addr(),
		}
	}

	fn reserve_validator_script() -> PlutusScript {
		PlutusScript::v3_from_cbor(&hex!(
			"591dff010100229800aba4aba2aba1aba0aab9faab9eaab9dab9cab9a9bae0024888888888a60022a6600692013165787065637420726573657276655f6f75747075745f646174756d3a2057696b7361203d206f75747075745f646174756d00168a99801a493465787065637420496e6c696e65446174756d286f75747075745f646174756d29203d206f75747075745f7574786f2e646174756d00168a99801a4926657870656374205b6f75747075745f7574786f5d203d20726573657276655f6f75747075747300168a99801a49ff657870656374205b676f7665726e616e63655265666572656e6365496e7075745d203d0a2020202020202020202073656c662e7265666572656e63655f696e707574730a2020202020202020202020207c3e206c6973742e66696c746572280a20202020202020202020202020202020666e28696e70757429207b0a2020202020202020202020202020202020206c6574206f7574707574203d20696e7075742e6f75747075740a2020202020202020202020202020202020206c6574206861735f746f6b656e203d0a20202020202020202020202020202020202020206173736574732e746f5f64696374286f75747075742e76616c7565290a20202020ff2020202020202020202020202020202020207c3e20646963742e6765742876657273696f6e5f6f7261636c655f636f6e666967290a202020202020202020202020202020202020202020207c3e206f7074696f6e2e616e645f7468656e28646963742e676574285f2c202256657273696f6e206f7261636c652229290a202020202020202020202020202020202020202020207c3e206f7074696f6e2e69735f736f6d6528290a0a2020202020202020202020202020202020207768656e206f75747075742e646174756d206973207b0a2020202020202020202020202020202020202020496e6c696e65446174756d286461746129202d3e207b0a202020ff2020202020202020202020202020202020202020206966206461746120697320646174756d3a2056657273696f6e696e67446174756d207b0a2020202020202020202020202020202020202020202020206861735f746f6b656e20262620646174756d2e7363726970744964203d3d2033320a202020202020202020202020202020202020202020207d20656c7365207b0a20202020202020202020202020202020202020202020202046616c73650a202020202020202020202020202020202020202020207d0a202020202020202020202020202020202020202020207d0a20202020202020202020202020202020202020205f202d3e2046616c73650a362020202020202020202020202020202020207d0a202020202020202020202020202020207d2c0a20202020202020202020202020202900168a99801a49ff657870656374205b6f75747075745f6963735f7574786f5d203d0a20202020202020202020202020202020202073656c662e6f7574707574730a20202020202020202020202020202020202020207c3e206c6973742e66696c746572280a202020202020202020202020202020202020202020202020666e286f757470757429207b0a20202020202020202020202020202020202020202020202020206c65742061646472203d206f75747075742e616464726573730a2020202020202020202020202020202020202020202020202020616464722e7061796d656e745f63726564656e7469616c203d3d20536372697074280a2020202020202020202020882020202020202020202020202020202020696c6c69717569645f63697263756c6174696f6e5f737570706c795f7363726970745f686173682c0a2020202020202020202020202020202020202020202020202020290a2020202020202020202020202020202020202020202020207d2c0a202020202020202020202020202020202020202020202900168a99801a49ff657870656374205b696c6c697175696443697263756c6174696f6e537570706c795265666572656e6365496e7075745d203d0a2020202020202020202073656c662e7265666572656e63655f696e707574730a2020202020202020202020207c3e206c6973742e66696c746572280a20202020202020202020202020202020666e28696e70757429207b0a2020202020202020202020202020202020206c6574206f7574707574203d20696e7075742e6f75747075740a2020202020202020202020202020202020206c6574206861735f746f6b656e203d0a20202020202020202020202020202020202020206173736574732e746f5f64696374286f7574ff7075742e76616c7565290a202020202020202020202020202020202020202020207c3e20646963742e6765742876657273696f6e5f6f7261636c655f636f6e666967290a202020202020202020202020202020202020202020207c3e206f7074696f6e2e616e645f7468656e28646963742e676574285f2c202256657273696f6e206f7261636c652229290a202020202020202020202020202020202020202020207c3e206f7074696f6e2e69735f736f6d6528290a0a2020202020202020202020202020202020207768656e206f75747075742e646174756d206973207b0a2020202020202020202020202020202020202020496e6c696e65446174756dff286461746129202d3e207b0a2020202020202020202020202020202020202020202020206966206461746120697320646174756d3a2056657273696f6e696e67446174756d207b0a2020202020202020202020202020202020202020202020206861735f746f6b656e20262620646174756d2e7363726970744964203d3d2033300a202020202020202020202020202020202020202020207d20656c7365207b0a20202020202020202020202020202020202020202020202046616c73650a202020202020202020202020202020202020202020207d0a202020202020202020202020202020202020202020207d0a2020202020202020202020202020202045202020205f202d3e2046616c73650a2020202020202020202020202020202020207d0a202020202020202020202020202020207d2c0a20202020202020202020202020202900168a99801a492f65787065637420726573657276655f696e7075745f646174756d3a2057696b7361203d20696e7075745f646174756d00168a99801a49ff657870656374205b7265736572766541757468546f6b656e5265666572656e6365496e7075745d203d0a20202020202073656c662e7265666572656e63655f696e707574730a20202020202020207c3e206c6973742e66696c746572280a202020202020202020202020666e28696e70757429207b0a20202020202020202020202020206c6574206f7574707574203d20696e7075742e6f75747075740a20202020202020202020202020206c6574206861735f746f6b656e203d0a202020202020202020202020202020206173736574732e746f5f64696374286f75747075742e76616c7565290a2020202020202020202020202020202020207c3e2064ff6963742e6765742876657273696f6e5f6f7261636c655f636f6e666967290a2020202020202020202020202020202020207c3e206f7074696f6e2e616e645f7468656e28646963742e676574285f2c202256657273696f6e206f7261636c652229290a2020202020202020202020202020202020207c3e206f7074696f6e2e69735f736f6d6528290a0a20202020202020202020202020207768656e206f75747075742e646174756d206973207b0a20202020202020202020202020202020496e6c696e65446174756d286461746129202d3e207b0a20202020202020202020202020202020202020206966206461746120697320646174756d3a20566572e773696f6e696e67446174756d207b0a20202020202020202020202020202020202020206861735f746f6b656e20262620646174756d2e7363726970744964203d3d2032390a2020202020202020202020202020202020207d20656c7365207b0a202020202020202020202020202020202020202046616c73650a2020202020202020202020202020202020207d0a2020202020202020202020202020202020207d0a202020202020202020202020202020205f202d3e2046616c73650a20202020202020202020202020207d0a2020202020202020202020207d2c0a202020202020202020202900168a99801a493365787065637420726573657276655f72656465656d65723a205265736572766552656465656d6572203d2072656465656d657200164888888889660026465300130130019809980a000cdc3a400530130024888966002600460266ea800e2646644a6602a9210648454c4c4f3500159800803c4cc88cc89660026006003159800980d9baa00b801403901c45660026012003159800980d9baa00b801403901c45660026008003159800980d9baa00b801403901c456600266e1d20060018acc004c06cdd5005c00a01c80e201c80c1018203040603300122259800801c006264b3001001801400a0051332259800800c012264b3001001802c01600b0058992cc004c09400e00f00640886eb80050251811000a040375a00260420090024088603e00680ea4603a603c603c0032598009801180c9baa0018a518a50405d374a900048966002601260346ea800a298103d87a8000898009bab301e301b375400480c244646600200200644b30010018a5eb8226644b3001300500289981080119802002000c4cc01001000501c18100009810800a03c9180e980f000a4444444664464b300100180bc4c966002605000519800912cc004c050c094dd50014400626eb8c0a4c098dd5001204691814181498149814800c89660026028604a6ea800a20031375a6052604c6ea800902348c0a0c0a4c0a4c0a4c0a400664660020026eb0c0a0c094dd5009112cc0040062980103d87a80008992cc004cdd7981518139baa0010128980599814800a5eb8226600600660560048120c0a400502724444464b30013012302937540051323259800980d18159baa001899912cc004c05cc0b4dd5000c4c96600200302481244cc8a6002005198009bae30343031375460686eb0c0d000a6eb8c048c0c4dd5181a1bac3034002998059980a198081bab300a3031375403c6eb8c0d0dd618091bac3034002233012001488100480026eb4c05cdd6181a0014966002603460626ea80062946294102f4888c8cc00400401089660020031004899801981c80099801001181d000a06e98181baa0219b87481012222222232598009811001c56600201d0338992cc004c0fc03e330013375e604260766ea8044c084c0ecdd5000ccc054cc078cc068dd5980e181d9baa00100a23301c00100a480026603a6eb0c070c0ecdd501411919912cc004c0a0c0f8dd5000c4cc8966002005001800c4ca6002003159800802c4c028dd69822800c528207e80120523758005001800a08c3042303f37540022942294103c1811198101980e1bab301e303d375400207246603c00291010e56657273696f6e206f7261636c65003023303d3754002603a60786ea800522259800800c0da264b300130430028acc0056600260146603c6eacc060c0fcdd50161980d980d181f9baa3020303f3754002a6607a921106d697373696e6720736372697074203200168a518a9981ea49176d696e7473476f7665726e616e6365203f2046616c73650014a081e22b3001598008024528c54cc0f524011d646174756d5f646f65735f6e6f745f6368616e6765203f2046616c73650014a081e22b30013371200c00714a3153303d4901356f75747075745f726573657276655f746f6b656e73203e3d20696e7075745f726573657276655f746f6b656e73203f2046616c73650014a081e2294103c4528207881ba080304100140fc81a2078303d00e40ed132598009814802456600201f0348992cc004c100042264b30013026303c375400313259800800c0e60731329800800c6600266e1ccdc080580619b8133019330223301e37566040607e6ea80100388cc0800040392000006acc004cdd798211bac304200130423758608402113375e60406eb0c108004c080dd6182100845282078998109bac3020303f37540584646644b3001302c30423754003133225980080140060031329800800c56600200b13370e6eb4c124005203c8a50410d00240b46eb000a0030014128608c60866ea800452845282080302633024330203756604460826ea80040f48cc08800522010e56657273696f6e206f7261636c6500302730413754002604260806ea800522259800800c0e2264b30013047002899192cc0040060791325980098250014566002b30010068a518a99822248122646174756d5f6368616e67655f6f6e6c795f62795f7374617473203f2046616c73650014a0821a2b300159800803c528c54cc11124012661737365745f6368616e67655f62795f636f72726563745f616d6f756e74203f2046616c73650014a0821a2b30013370e66e00cc080cc0a4cc094dd5981398231baa00101523302700101548000dd698139bac30273758609202e66e00cdc08098094c004cc0a0dd6182498231baa03323375e6094608e6ea8c128c11cdd5181418239baa001300d33049375200897ae0a4001223370000266044660566604e6eacc0a4c120dd5181498241baa002017233029001017480010104528c54cc111240129636f72726563745f616d6f756e745f7472616e736665727265645f746f5f696373203f2046616c73650014a0821a29410434528208681ea08e304800141186604c6eb0c0a8c110dd5018919baf3048304537546090608a6ea8004c02ccc11cdd480125eb80cc07cc078c10cdd5181218219baa001533041491106d697373696e67207363726970742033001681ca0883045001410c81d201e375800303981ca0863040303d375400303740e8604460786ea800606a81e8c0f803d03c45660026048009159800807c0d2264b300130400108992cc004c098c0f0dd5000c4c96600200303981cc4ca6002003198009980c998111980f1bab3020303f375400801c46604000201c900056600266ebcc108dd6182100098211bac3042010899b87375a604a6eb0c108004dd698129bac30420108a5040f13302137586040607e6ea80b08c8cc8966002605860846ea800626644b3001002800c00626530010018acc0040162601c6eb4c1240062941043400902d1bac002800c00504a182318219baa00114a114a08200c098cc090cc080dd5981118209baa00103d23302200148810e56657273696f6e206f7261636c6500302730413754002604260806ea800522259800800c0ea264b300130470028acc00566002601c660446eacc070c10cdd50181980f980f18219baa302430433754002a66082921106d697373696e6720736372697074203400168a518a99820a49176d696e7473476f7665726e616e6365203f2046616c73650014a082022b300159800801c528c54cc10524012d646174756d5f6368616e67655f6f6e6c795f62795f6d757461626c655f73657474696e6773203f2046616c73650014a082022b30013370e00801514a315330414901356f75747075745f726573657276655f746f6b656e73203d3d20696e7075745f726573657276655f746f6b656e73203f2046616c73650014a0820229410404528208081da0883045001410c81d201e375800303981ca0863040303d375400303740e8604460786ea800606a81e8c0f803d03c456600201f13259800800c0ca264b30013041002899192cc00400606d13259800982200144cc896600200303a8992cc004c11c00a2b3001598009807198111bab301c304337540606603e603c60866ea8c090c10cdd5000a99820a481106d697373696e6720736372697074203600168a518a99820a49176d696e7473476f7665726e616e6365203f2046616c73650014a082022b30015980080d4528c54cc10524011a6275726e735f726573657276655f61757468203f2046616c73650014a082022b30010038a518a99820a4812c616c6c5f726573657276655f746f6b656e735f7472616e7366657265645f746f5f696373203f2046616c73650014a0820229410404528208081da0883045001410c66e1ccdc04c004cc088dd6182198201baa02d23375e608860826ea8c110c104dd5181118209baa001300733043375200897ae0a40012233700002660386604a660426eacc08cc108dd5181198211baa0020112330230010114800100a0039980d198119980f9bab30213040375400201e46604200201e9000198111bac30213040375405a4646644b3001302d30433754003133225980080140060031329800800c56600200b1300f375a609400314a082220048170dd600140060028258c11cc110dd50008a508a504104604e6604a660426eacc08cc108dd500081f119811800a450e56657273696f6e206f7261636c6500302830423754002604460826ea800606e8208c108005040198101bac3024303e3754056466ebcc108c0fcdd51821181f9baa001300533041375200497ae0330193018303d3754603c607a6ea80054cc0ed2401106d697373696e672073637269707420350016819a07c303f00140f46603a6eb0c070c0ecdd501411919912cc004c0a0c0f8dd5000c4cc8966002005001800c4ca6002003159800802c4cdc39bad3045001480f2294103f40090291bac002800c0050461821181f9baa00114a114a081e0c088cc080cc070dd5980f181e9baa00103923301e00148810e56657273696f6e206f7261636c65003023303d3754002603a60786ea80062a660729211b657870656374205b5d203d20726573657276655f6f757470757473001640f081c10381ba548009037198099980e1980c1bab301a3039375401e0104660340020109000204c8008dd60009112cc00400e00313259800800c00a0051332259800800c01226644b300100180344c966002003007803c01e264b3001303d0038acc00401a01113259800800c4c96600200300a8992cc00400601700b899912cc00400601b13259800800c03a01d00e8992cc004c11000e2b30013028303f375400f13259800800c042264b3001001808c046023011899912cc00400602713259800800c05202901480a44c96600260940071598008054056264b300100180b405a02d016899912cc00400603113259800800c0660330198992cc004c13c00e03701a41306eb40060328278c13000504a1bae001304b00b41306092014823a02a8238dd7000a094304700141146eb8004c1180090471822000a0843040375400f00f40f500f41046eb400601c8220c10400503f1bac0013040002805c02d041181f000a078303e007804c02601300940fc607800c81d201081d0dd6800c01d03d181d000a0703038001303900140d86eb0004c0dc01200500240e0606a006819a04902440d06062605c6ea80062a6605892013265787065637420496e6c696e65446174756d28696e7075745f646174756d29203d20696e7075745f7574786f2e646174756d001640ac664466020004464b3001301e302f375400313371e0066eb8c0ccc0c0dd5000c528205a3032302f37546064605e6ea8004dd6180918161baa019375c605e60586ea8004c048c0b0dd5001454cc0a924014a6578706563742053637269707428726573657276655f7363726970745f6861736829203d20696e7075745f7574786f2e616464726573732e7061796d656e745f63726564656e7469616c001640a4605c60566ea8c0b8c0acdd5000980598151baa302d302a3754005153302849014e65787065637420536f6d6528696e70757429203d0a20202020202073656c662e696e707574730a20202020202020207c3e207472616e73616374696f6e2e66696e645f696e707574287574786f290016409c66e1d2001330033300c330083756600460526ea8058cc014c010c0a4dd5180518149baa0065330274901106d697373696e67207363726970742031001623300a00148900480010184094604c0028120cc010dd6180198111baa00f232332259800980798129baa001899912cc00400a0030018994c0040062b3001005899b87375a6058002901d4528204c80120203758005001800a05a30293026375400229422941023180499803998019bab30053024375400204046600a0029110e56657273696f6e206f7261636c6500300a30243754002600860466ea800488c8cc00400400c896600200314c0103d87a8000899192cc004cdc8802800c56600266e3c0140062601466050604c00497ae08a60103d87a8000408d133004004302a003408c6eb8c090004c09c005025111919800800801912cc0040062980103d87a8000899192cc004cdc8802800c56600266e3c0140062601466050604c00497ae08a60103d87a8000408d133004004302a003408c6eb8c090004c09c0050250c060dd50049b8748010dc3a400100a805402a01480e0c060004c060c064004c050dd5001c590110c04c004c038dd500a45268a998062491856616c696461746f722072657475726e65642066616c73650013656402c1"
		)).unwrap()
	}

	fn auth_policy_script() -> PlutusScript {
		RESERVE_AUTH_POLICY.into()
	}

	fn illiquid_supply_validator_script() -> PlutusScript {
		ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR.into()
	}

	fn illiquid_supply_auth_token_policy_script() -> PlutusScript {
		ILLIQUID_CIRCULATION_SUPPLY_AUTHORITY_TOKEN_POLICY.into()
	}

	const UNIX_T0: u64 = 1736504093000u64;

	fn applied_v_function() -> PlutusScript {
		plutus_script![EXAMPLE_V_FUNCTION_POLICY, UNIX_T0].unwrap()
	}

	fn version_oracle_address() -> String {
		"addr_test1wqskkgpmsyf0yr2renk0spgsvea75rkq4yvalrpwwudwr5ga3relp".to_string()
	}

	fn ics_validator_address() -> String {
		"addr_test1wz7s8uzkldpv7rvyu6v8wg4vk3ulxy23kzqyngk2yc76xucestz8f".to_string()
	}

	fn reserve_data() -> ReserveData {
		ReserveData {
			scripts: ReserveScripts {
				validator: reserve_validator_script(),
				auth_policy: auth_policy_script(),
			},
			auth_policy_version_utxo: OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("d1030b0ce5cf33d97a6e8aafa1cfe150e7a8b3a5584bd7a345743938e78ec44b"),
				},
				index: 0,
				address: version_oracle_address(),
				script: Some(auth_policy_script().into()),
				..Default::default()
			},
			validator_version_utxo: OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("fcb5d7877e6ce7cfaef579c4f4b5fdbbdb807e4fe613752671742f1a5191c850"),
				},
				index: 0,
				address: version_oracle_address(),
				script: Some(reserve_validator_script().into()),
				..Default::default()
			},
		}
	}

	fn ics_data() -> ICSData {
		ICSData {
			scripts: ICSScripts {
				validator: illiquid_supply_validator_script(),
				auth_policy: illiquid_supply_auth_token_policy_script(),
			},
			auth_policy_version_utxo: OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("f5890475177fcc7cf40679974751f66331c7b25fcf2f1a148c53cf7e0e147114"),
				},
				index: 1,
				address: version_oracle_address(),
				script: Some(illiquid_supply_auth_token_policy_script().into()),
				..Default::default()
			},
			validator_version_utxo: OgmiosUtxo {
				transaction: OgmiosTx {
					id: hex!("f5890475177fcc7cf40679974751f66331c7b25fcf2f1a148c53cf7e0e147114"),
				},
				index: 0,
				address: version_oracle_address(),
				script: Some(illiquid_supply_validator_script().into()),
				..Default::default()
			},
		}
	}

	fn reference_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("45882cfd2de9381f34ae68ad073452e2a57a7ad11095dae49f365266637e9d04"),
			},
			index: 0,
			script: Some(applied_v_function().into()),
			..Default::default()
		}
	}

	fn previous_reserve_ogmios_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("23de508bbfeb6af651da305a2de022463f71e47d58365eba36d98fa6c4aed731"),
			},
			value: OgmiosValue {
				lovelace: 1672280,
				native_tokens: [
					(
						// reserve token
						token_policy().0,
						vec![Asset { name: token_name().0.to_vec(), amount: 990 }],
					),
					(
						// leftover governance token - should be returned to the validator
						hex!("75b8875ff8958c66fecbd93740ac5ffd7370d299e729a46bb5632066"),
						vec![Asset { name: vec![], amount: 1 }],
					),
				]
				.into(),
			},

			index: 1,
			..Default::default()
		}
	}

	fn previous_reserve_utxo() -> ReserveUtxo {
		ReserveUtxo { utxo: previous_reserve_ogmios_utxo(), datum: previous_reserve_datum() }
	}

	fn ics_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("23de508bbfeb6af651da305a2de022463f71e47d58365eba36d98fa6c4aed731"),
			},
			value: OgmiosValue {
				lovelace: 1672280,
				native_tokens: [(
					// reserve token
					ics_auth_token_policy().0,
					vec![Asset { name: vec![], amount: 1 }],
				)]
				.into(),
			},

			index: 2,
			address: ics_validator_address(),
			..Default::default()
		}
	}

	fn token_policy() -> PolicyId {
		PolicyId(hex!("1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2b"))
	}

	fn ics_auth_token_policy() -> PolicyId {
		PolicyId(hex!("d4a1d93484fa63847f8c4c271dc0c46a55b6e5916f46e14ee849a381"))
	}

	fn token_name() -> AssetName {
		AssetName::from_hex_unsafe("52657761726420746f6b656e")
	}

	fn token_id() -> sidechain_domain::AssetId {
		sidechain_domain::AssetId { policy_id: token_policy(), asset_name: token_name() }
	}

	fn previous_reserve_datum() -> ReserveDatum {
		ReserveDatum {
			immutable_settings: ReserveImmutableSettings { token: token_id() },
			mutable_settings: ReserveMutableSettings {
				total_accrued_function_asset_name: applied_v_function().policy_id(),
				initial_incentive: 0,
			},
			stats: ReserveStats { token_total_amount_transferred: 10 },
		}
	}

	fn reserve_release_test_tx() -> Transaction {
		reserve_release_tx(
			&tx_context(),
			&reserve_data(),
			&ics_data(),
			&previous_reserve_utxo(),
			&ics_utxo(),
			&reference_utxo(),
			5,
			0,
			Costs::ZeroCosts,
		)
		.unwrap()
	}

	#[test]
	fn should_have_correct_reference_utxos() {
		let ref_inputs: Vec<_> = reserve_release_test_tx()
			.body()
			.reference_inputs()
			.expect("Should have reference inputs")
			.into_iter()
			.cloned()
			.collect();

		assert!(ref_inputs.contains(&reference_utxo().to_csl_tx_input()));
		assert!(ref_inputs.contains(&reserve_data().auth_policy_version_utxo.to_csl_tx_input()));
		assert!(ref_inputs.contains(&reserve_data().validator_version_utxo.to_csl_tx_input()));
		assert!(ref_inputs.contains(&ics_data().auth_policy_version_utxo.to_csl_tx_input()));
		assert!(ref_inputs.contains(&ics_data().validator_version_utxo.to_csl_tx_input()));
		assert_eq!(ref_inputs.len(), 5)
	}

	#[test]
	fn should_mint_v_function_scripts() {
		let v_function_token_mint_amount = reserve_release_test_tx()
			.body()
			.mint()
			.expect("Should mint a token")
			.get(&applied_v_function().csl_script_hash())
			.and_then(|policy| policy.get(0))
			.expect("Should mint a v-function policy token")
			.get(&empty_asset_name())
			.expect("The minted token should have an empty asset name");

		assert_eq!(v_function_token_mint_amount, Int::new_i32(15))
	}

	#[test]
	fn should_burn_previoius_reserve_utxo() {
		let inputs: Vec<_> =
			reserve_release_test_tx().body().inputs().into_iter().cloned().collect();

		assert!(inputs.contains(&previous_reserve_utxo().utxo.to_csl_tx_input()))
	}

	#[test]
	fn should_add_correct_number_of_tokens_to_illiquid_supply() {
		let illiquid_supply_output = (reserve_release_test_tx().body().outputs().into_iter())
			.find(|output| {
				output.address()
					== illiquid_supply_validator_script().address(NetworkIdKind::Testnet)
			})
			.expect("Should output a UTXO to illiquid supply validator")
			.amount()
			.multiasset()
			.expect("Should output native tokens to illiquid supply")
			.get(&PolicyID::from(token_policy().0))
			.expect("Should transfer reserve token policy token to illiquid supply")
			.get(&token_name().to_csl().unwrap())
			.expect("Should transfer reserve token to illiquid supply");

		assert_eq!(illiquid_supply_output, 5u64.into())
	}

	#[test]
	fn should_leave_unreleased_tokens_at_reserve_validator() {
		let validator_output = (reserve_release_test_tx().body().outputs().into_iter())
			.find(|output| {
				output.address() == reserve_validator_script().address(NetworkIdKind::Testnet)
			})
			.expect("Should output a UTXO to illiquid supply validator")
			.amount()
			.multiasset()
			.expect("Should output native tokens to illiquid supply")
			.get(&PolicyID::from(token_policy().0))
			.expect("Should transfer reserve token policy token to illiquid supply")
			.get(&token_name().to_csl().unwrap())
			.expect("Should transfer reserve token to illiquid supply");

		assert_eq!(validator_output, 985u64.into())
	}
}
