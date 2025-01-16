//! Transaction that updates reserve settings.
//!
//! Specification:
//! 1. The transaction should mint one token:
//!   * 1 Governance Policy Token (using reference script)
//! 2. The transaction should spend one token:
//!   * 1 Reserve Auth Policy Token (using reference script)
//! 3. The transaction should have two outputs:
//!   * Reserve Validator output that:
//!   * * has value from existing reserve UTXO
//!   * * has the updated Plutus Data (in our "versioned format"): `[[[Int(t0), <Encoded Token>], [Bytes(v_function_hash), Int(initial_incentive)], [Int(0)]], Constr(0, []), Int(0)]`,
//!       where `<Encoded Token>` is `Constr(0, [Bytes(policy_id), Bytes(asset_name)])`.
//!   * Change output that keeps the Governance Token and change of other tokens
//! 4. The transaction should have three script reference inputs:
//!   * Reserve Auth Version Utxo
//!   * Reserve Validator Version Utxo
//!   * Governance Policy Script

use super::ReserveData;
use crate::{csl::*, init_governance::GovernanceData};
use cardano_serialization_lib::*;
use partner_chains_plutus_data::reserve::{ReserveDatum, ReserveRedeemer};

fn update_reserve_settings_tx(
	datum: &ReserveDatum,
	reserve: &ReserveData,
	governance: &GovernanceData,
	governance_script_cost: ExUnits,
	redeemer_script_cost: ExUnits,
	ctx: &TransactionContext,
) -> Result<Transaction, JsError> {
	let mut tx_builder = TransactionBuilder::new(&get_builder_config(ctx)?);

	// spend old settings
	{
		let mut inputs = TxInputsBuilder::new();

		let utxo = &reserve.validator_version_utxo;
		let input = utxo.to_csl_tx_input();
		let amount = &utxo.value.to_csl()?;
		let witness = PlutusWitness::new_with_ref_without_datum(
			&PlutusScriptSource::new_ref_input(
				&reserve.scripts.validator.csl_script_hash(),
				&reserve.validator_version_utxo.to_csl_tx_input(),
				&reserve.scripts.validator.language,
				reserve.scripts.validator.bytes.len(),
			),
			&Redeemer::new(
				&RedeemerTag::new_spend(),
				// CSL will set redeemer index for the index of script input after sorting transaction inputs
				&0u32.into(),
				&ReserveRedeemer::UpdateReserve { governance_version: 1u64 }.into(),
				&redeemer_script_cost,
			),
		);
		inputs.add_plutus_script_input(&witness, &input, amount);

		tx_builder.set_inputs(&inputs);
	}
	// mint new settings
	{
		let amount_builder = TransactionOutputBuilder::new()
			.with_address(&reserve.scripts.validator.address(ctx.network))
			.with_plutus_data(&(datum.clone().into()))
			.with_script_ref(&ScriptRef::new_plutus_script(&reserve.scripts.validator.to_csl()))
			.next()?;
		let mut val = reserve.validator_version_utxo.value.to_csl()?;
		let output = amount_builder.with_value(&val).build()?;
		let min_ada = MinOutputAdaCalculator::new(
			&output,
			&DataCost::new_coins_per_byte(
				&ctx.protocol_parameters.min_utxo_deposit_coefficient.into(),
			),
		)
		.calculate_ada()?;
		val.set_coin(&min_ada);
		let a = amount_builder.with_value(&val).build()?;
		tx_builder.add_output(&a)?;
	}

	let gov_tx_input = governance.utxo_id_as_tx_input();
	tx_builder.add_mint_one_script_token_using_reference_script(
		&governance.policy_script,
		&gov_tx_input,
		&governance_script_cost,
	)?;
	tx_builder.add_script_reference_input(
		&reserve.illiquid_circulation_supply_validator_version_utxo.to_csl_tx_input(),
		reserve.scripts.illiquid_circulation_supply_validator.bytes.len(),
	);
	tx_builder.balance_update_and_build(ctx)
}

#[cfg(test)]
mod tests {

	use cardano_serialization_lib::{ExUnits, Language, NetworkIdKind};
	use hex_literal::hex;
	use ogmios_client::types::{OgmiosTx, OgmiosUtxo};
	use partner_chains_plutus_data::reserve::ReserveDatum;
	use sidechain_domain::{AssetId, AssetName, PolicyId};

	use super::update_reserve_settings_tx;
	use crate::{
		csl::TransactionContext,
		init_governance::GovernanceData,
		plutus_script::PlutusScript,
		reserve::ReserveData,
		scripts_data::ReserveScripts,
		test_values::{make_utxo, payment_addr, payment_key, protocol_parameters},
	};

	const REWARDS_TOKEN_POLICY_ID: PolicyId =
		PolicyId(hex!("1fab25f376bc49a181d03a869ee8eaa3157a3a3d242a619ca7995b2b"));

	// Reward token
	const REWARDS_TOKEN_ASSET_NAME_STR: &str = "52657761726420746f6b656e";

	#[test]
	fn update_reserve_settings_tx_test() {
		let parameters = crate::reserve::create::ReserveParameters {
			initial_incentive: 100,
			total_accrued_function_script_hash: PolicyId([233u8; 28]),
			token: AssetId {
				policy_id: REWARDS_TOKEN_POLICY_ID,
				asset_name: AssetName::from_hex_unsafe(REWARDS_TOKEN_ASSET_NAME_STR),
			},
			initial_deposit: 500000,
		};

		let reserve: ReserveData = ReserveData {
			scripts: ReserveScripts {
				validator: test_validator_script(),
				auth_policy: test_auth_policy_script(),
				illiquid_circulation_supply_validator: test_ics_validator_script(),
			},
			auth_policy_version_utxo: test_auth_policy_version_utxo(),
			validator_version_utxo: test_validator_version_utxo(),
			illiquid_circulation_supply_validator_version_utxo: test_ics_validator_version_utxo(),
		};
		let tx = update_reserve_settings_tx(
			&(&parameters).into(),
			&reserve,
			&test_governance_data(),
			governance_script_cost(),
			redeemer_script_cost(),
			&test_transaction_context(),
		)
		.unwrap();

		let body = tx.body();
		let inputs = body.inputs();
		let outputs = body.outputs();

		// Both inputs are used to cover transaction
		assert_eq!(
			inputs.get(0).to_string(),
			"1212121212121212121212121212121212121212121212121212121212121212#0"
		);
		assert_eq!(
			inputs.get(1).to_string(),
			"7474747474747474747474747474747474747474747474747474747474747474#3"
		);
		assert_eq!(
			outputs.get(0).address(),
			test_validator_script().address(cardano_serialization_lib::NetworkIdKind::Testnet)
		);
		assert_eq!(
			outputs.get(1).address().to_hex(),
			"6032230eeaaae0ff7a97e7f088e65874bd79c0fe2a99399f795e84543a"
		);
		assert_eq!(outputs.get(0).plutus_data().unwrap(), ReserveDatum::from(&parameters).into());
	}

	fn test_transaction_context() -> TransactionContext {
		TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: vec![make_utxo(116u8, 3, 996272387, &payment_addr())],
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		}
	}

	fn test_validator_script() -> PlutusScript {
		PlutusScript { bytes: hex!("445566").to_vec(), language: Language::new_plutus_v2() }
	}

	fn test_auth_policy_script() -> PlutusScript {
		PlutusScript { bytes: hex!("556677").to_vec(), language: Language::new_plutus_v2() }
	}

	fn test_ics_validator_script() -> PlutusScript {
		PlutusScript { bytes: hex!("667788").to_vec(), language: Language::new_plutus_v2() }
	}

	fn test_governance_script() -> PlutusScript {
		PlutusScript { bytes: hex!("112233").to_vec(), language: Language::new_plutus_v2() }
	}

	fn test_governance_input() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [16; 32] }, index: 0, ..Default::default() }
	}

	fn test_auth_policy_version_utxo() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [17; 32] }, index: 0, ..Default::default() }
	}

	fn test_validator_version_utxo() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [18; 32] }, index: 0, ..Default::default() }
	}

	fn test_ics_validator_version_utxo() -> OgmiosUtxo {
		OgmiosUtxo { transaction: OgmiosTx { id: [19; 32] }, index: 0, ..Default::default() }
	}

	fn test_governance_data() -> GovernanceData {
		GovernanceData { policy_script: test_governance_script(), utxo: test_governance_input() }
	}

	fn governance_script_cost() -> ExUnits {
		ExUnits::new(&100u64.into(), &200u64.into())
	}

	fn redeemer_script_cost() -> ExUnits {
		ExUnits::new(&300u64.into(), &400u64.into())
	}
}
