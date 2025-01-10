use crate::{
	csl::*,
	plutus_script::PlutusScript,
	scripts_data::{multisig_governance_policy_configuration, version_oracle},
};
use cardano_serialization_lib::*;
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use sidechain_domain::MainchainAddressHash;

// Script ID of the governance script in the script cache.
// TODO: Use a proper value of raw_scripts::ScripId once we upgrade to a version that has it.
const SCRIPT_ID: u32 = 32;

pub(crate) fn init_governance_transaction(
	governance_authority: MainchainAddressHash,
	tx_context: &TransactionContext,
	genesis_utxo: OgmiosUtxo,
	ex_units: ExUnits,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::MULTI_SIG_POLICY, LanguageKind::PlutusV2)?
			.apply_uplc_data(multisig_governance_policy_configuration(governance_authority))?;
	let version_oracle = version_oracle(genesis_utxo.to_domain(), tx_context.network)?;
	let config = crate::csl::get_builder_config(tx_context)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	tx_builder.set_mint_builder(&{
		let mut mint_builder = MintBuilder::new();

		mint_builder.add_asset(
			&mint_witness(&version_oracle.policy, &multi_sig_policy, &ex_units)?,
			&version_oracle_asset_name(),
			&Int::new_i32(1),
		)?;
		mint_builder
	});

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle.validator.clone(),
		version_oracle.policy.clone(),
		multi_sig_policy,
		tx_context.network,
		tx_context,
	)?)?;

	tx_builder.set_inputs(&{
		TxInputsBuilder::with_key_inputs(&[genesis_utxo], &tx_context.payment_key_hash())?
	});

	Ok(tx_builder.balance_update_and_build(tx_context)?)
}

fn version_oracle_asset_name() -> AssetName {
	AssetName::new(b"Version oracle".to_vec()).expect("Constant asset name should work")
}

fn mint_witness(
	version_oracle_policy: &PlutusScript,
	multi_sig_policy: &PlutusScript,
	ex_units: &ExUnits,
) -> anyhow::Result<MintWitness> {
	Ok(MintWitness::new_plutus_script(
		&PlutusScriptSource::new(&version_oracle_policy.to_csl()),
		&Redeemer::new(
			&RedeemerTag::new_mint(),
			&0u32.into(),
			&PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&0u64.into(), &{
				let mut list = PlutusList::new();
				list.add(&PlutusData::new_integer(&SCRIPT_ID.into()));
				list.add(&PlutusData::new_bytes(multi_sig_policy.script_hash().to_vec()));
				list
			})),
			ex_units,
		),
	))
}

pub(crate) fn version_oracle_datum_output(
	version_oracle_validator: PlutusScript,
	version_oracle_policy: PlutusScript,
	multi_sig_policy: PlutusScript,
	network: NetworkIdKind,
	tx_context: &TransactionContext,
) -> anyhow::Result<cardano_serialization_lib::TransactionOutput> {
	let datum: PlutusData = VersionOracleDatum {
		version_oracle: SCRIPT_ID,
		currency_symbol: version_oracle_policy.policy_id().0,
	}
	.into();

	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&version_oracle_validator.address(network))
		.with_plutus_data(&datum)
		.with_script_ref(&ScriptRef::new_plutus_script(&multi_sig_policy.to_csl()))
		.next()?;
	let mut ma = MultiAsset::new();
	let mut assets = Assets::new();
	assets.insert(&version_oracle_asset_name(), &1u64.into());
	ma.insert(&version_oracle_policy.policy_id().0.into(), &assets);
	let output = amount_builder.with_coin_and_asset(&0u64.into(), &ma).build()?;
	let min_ada = MinOutputAdaCalculator::new(
		&output,
		&DataCost::new_coins_per_byte(
			&tx_context.protocol_parameters.min_utxo_deposit_coefficient.into(),
		),
	)
	.calculate_ada()?;
	let output = amount_builder.with_coin_and_asset(&min_ada, &ma).build()?;
	Ok(output)
}
