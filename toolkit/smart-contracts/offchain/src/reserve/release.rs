//! This transaction releases some funds to the illiquid supply.
//! Inputs:
//!     - previous utxo at the *reserve validator*, containing the reserve tokens and the
//!       [ReserveData] plutus data with reserve configuration and release stats
//! Reference inputs:
//!     - utxo with V-Function reference script matching the hash saved in the input [ReserveData].
//!       IMPORTANT: The V-Function script will evaluate against the total number of tokens that
//!                  should have been released up to now.
//!                  The number of tokens released in a single transaction equals
//!                  <current v-function value> - <number of previously released tokens>.
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
use super::{reserve_utxo_input_with_validator_script_reference, ReserveData};
use crate::{
	await_tx::AwaitTx, cardano_keys::CardanoPaymentSigningKey, csl::*, plutus_script::PlutusScript,
	reserve::ReserveUtxo,
};
use anyhow::anyhow;
use cardano_serialization_lib::{
	Int, MultiAsset, PlutusData, Transaction, TransactionBuilder, TransactionOutputBuilder,
};
use ogmios_client::{
	query_ledger_state::*, query_network::QueryNetwork, transactions::Transactions,
	types::OgmiosUtxo,
};
use partner_chains_plutus_data::reserve::ReserveRedeemer;
use sidechain_domain::{McTxHash, UtxoId};
use std::num::NonZero;

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

	let reserve_utxo = reserve_data.get_reserve_utxo(&ctx, client).await?;

	let tx = Costs::calculate_costs(
		|costs| {
			reserve_release_tx(
				&ctx,
				&reserve_data,
				&reserve_utxo,
				&reference_utxo,
				amount.into(),
				tip.slot,
				costs,
			)
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
	await_tx.await_tx_output(client, UtxoId::new(tx_id, 0)).await?;

	Ok(McTxHash(tx_id))
}

fn reserve_release_tx(
	ctx: &TransactionContext,
	reserve_data: &ReserveData,
	previous_reserve: &ReserveUtxo,
	reference_utxo: &OgmiosUtxo,
	amount_to_transfer: u64,
	latest_slot: u64,
	costs: Costs,
) -> anyhow::Result<Transaction> {
	let token = &previous_reserve.datum.immutable_settings.token;
	let stats = &previous_reserve.datum.stats;

	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	let reserve_balance = previous_reserve.utxo.get_asset_amount(token);
	let token_total_amount_transferred = stats.token_total_amount_transferred;
	let cumulative_total_transfer: u64 = token_total_amount_transferred
		.checked_add(amount_to_transfer)
		.expect("cumulative_total_transfer can't overflow u64");

	let left_in_reserve = reserve_balance.checked_sub(amount_to_transfer)
		.ok_or_else(||anyhow!("Not enough funds in the reserve to transfer {amount_to_transfer} tokens (reserve balance: {reserve_balance})"))?;

	// Additional reference scripts
	tx_builder.add_script_reference_input(
		&reserve_data.auth_policy_version_utxo.to_csl_tx_input(),
		reserve_data.scripts.auth_policy.bytes.len(),
	);
	tx_builder.add_script_reference_input(
		&reserve_data
			.illiquid_circulation_supply_validator_version_utxo
			.to_csl_tx_input(),
		reserve_data.scripts.illiquid_circulation_supply_validator.bytes.len(),
	);

	// Mint v-function tokens in the number equal to the *total* number of tokens transfered.
	// This serves as a validation of the v-function value.
	let v_function = v_function_from_utxo(reference_utxo)?;
	tx_builder.add_mint_script_token_using_reference_script(
		&Script::Plutus(v_function),
		&reference_utxo.to_csl_tx_input(),
		&Int::new(&cumulative_total_transfer.into()),
		&costs,
	)?;

	// Remove tokens from the reserve
	tx_builder.set_inputs(&reserve_utxo_input_with_validator_script_reference(
		&previous_reserve.utxo,
		reserve_data,
		ReserveRedeemer::ReleaseFromReserve,
		&costs.get_one_spend(),
	)?);

	// Transfer released tokens to the illiquid supply
	tx_builder.add_output(&{
		TransactionOutputBuilder::new()
			.with_address(
				&reserve_data.scripts.illiquid_circulation_supply_validator.address(ctx.network),
			)
			.with_plutus_data(&PlutusData::new_empty_constr_plutus_data(&0u64.into()))
			.next()?
			.with_minimum_ada_and_asset(&token.to_multi_asset(amount_to_transfer)?, ctx)?
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
	use super::{empty_asset_name, reserve_release_tx, AssetNameExt, Costs, TransactionContext};
	use crate::{
		plutus_script::PlutusScript,
		reserve::{release::OgmiosUtxoExt, ReserveData, ReserveUtxo},
		scripts_data::ReserveScripts,
		test_values::{payment_addr, protocol_parameters},
	};
	use cardano_serialization_lib::{
		Int, Language, NetworkIdKind, PolicyID, PrivateKey, Transaction,
	};
	use hex_literal::hex;
	use ogmios_client::types::{Asset, OgmiosTx, OgmiosUtxo, OgmiosValue};
	use partner_chains_plutus_data::reserve::{
		ReserveDatum, ReserveImmutableSettings, ReserveMutableSettings, ReserveStats,
	};
	use pretty_assertions::assert_eq;
	use sidechain_domain::{AssetName, PolicyId};

	fn payment_key() -> PrivateKey {
		PrivateKey::from_normal_bytes(&hex!(
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
		PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_VALIDATOR, Language::new_plutus_v2())
			.unwrap()
	}

	fn auth_policy_script() -> PlutusScript {
		PlutusScript::from_wrapped_cbor(raw_scripts::RESERVE_AUTH_POLICY, Language::new_plutus_v2())
			.unwrap()
	}

	fn illiquid_supply_validator_script() -> PlutusScript {
		PlutusScript::from_wrapped_cbor(
			raw_scripts::ILLIQUID_CIRCULATION_SUPPLY_VALIDATOR,
			Language::new_plutus_v2(),
		)
		.unwrap()
	}

	const UNIX_T0: u64 = 1736504093000u64;

	fn applied_v_function() -> PlutusScript {
		PlutusScript::from_wrapped_cbor(
			raw_scripts::EXAMPLE_V_FUNCTION_POLICY,
			Language::new_plutus_v2(),
		)
		.unwrap()
		.apply_data(UNIX_T0)
		.unwrap()
	}

	fn version_oracle_address() -> String {
		"addr_test1wqskkgpmsyf0yr2renk0spgsvea75rkq4yvalrpwwudwr5ga3relp".to_string()
	}

	fn reserve_data() -> ReserveData {
		ReserveData {
			scripts: ReserveScripts {
				validator: reserve_validator_script(),
				auth_policy: auth_policy_script(),
				illiquid_circulation_supply_validator: illiquid_supply_validator_script(),
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
			illiquid_circulation_supply_validator_version_utxo: OgmiosUtxo {
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

	fn token_policy() -> PolicyId {
		PolicyId(hex!("1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2b"))
	}

	fn token_name() -> AssetName {
		AssetName::from_hex_unsafe("52657761726420746f6b656e")
	}

	fn token_id() -> sidechain_domain::AssetId {
		sidechain_domain::AssetId { policy_id: token_policy(), asset_name: token_name() }
	}

	fn previous_reserve_datum() -> ReserveDatum {
		ReserveDatum {
			immutable_settings: ReserveImmutableSettings { t0: 0, token: token_id() },
			mutable_settings: ReserveMutableSettings {
				total_accrued_function_script_hash: applied_v_function().policy_id(),
				initial_incentive: 0,
			},
			stats: ReserveStats { token_total_amount_transferred: 10 },
		}
	}

	fn reserve_release_test_tx() -> Transaction {
		reserve_release_tx(
			&tx_context(),
			&reserve_data(),
			&previous_reserve_utxo(),
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
		assert!(ref_inputs.contains(
			&reserve_data()
				.illiquid_circulation_supply_validator_version_utxo
				.to_csl_tx_input()
		));
		assert_eq!(ref_inputs.len(), 4)
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
