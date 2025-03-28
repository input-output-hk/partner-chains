use crate::{
	csl::*,
	plutus_script::PlutusScript,
	scripts_data::{multisig_governance_policy_configuration, version_oracle},
};
use cardano_serialization_lib::*;
use ogmios_client::types::OgmiosUtxo;
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use sidechain_domain::MainchainKeyHash;

pub(crate) fn init_governance_transaction(
	governance_authority: MainchainKeyHash,
	genesis_utxo: OgmiosUtxo,
	costs: Costs,
	ctx: &TransactionContext,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		PlutusScript::from_wrapped_cbor(raw_scripts::MULTI_SIG_POLICY, Language::new_plutus_v2())?
			.apply_uplc_data(multisig_governance_policy_configuration(governance_authority))?;
	let version_oracle = version_oracle(genesis_utxo.to_domain(), ctx.network)?;
	let config = crate::csl::get_builder_config(ctx)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	tx_builder.add_mint_one_script_token(
		&version_oracle.policy,
		&version_oracle_asset_name(),
		&mint_redeemer(&multi_sig_policy),
		&costs.get_mint(&version_oracle.policy),
	)?;

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle.validator.clone(),
		version_oracle.policy.clone(),
		multi_sig_policy,
		ctx.network,
		ctx,
	)?)?;

	tx_builder.set_inputs(&{ TxInputsBuilder::with_regular_inputs(&[genesis_utxo])? });

	Ok(tx_builder.balance_update_and_build(ctx)?)
}

fn version_oracle_asset_name() -> AssetName {
	AssetName::new(b"Version oracle".to_vec()).expect("Constant asset name should work")
}

fn mint_redeemer(multi_sig_policy: &PlutusScript) -> PlutusData {
	PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(&0u64.into(), &{
		let mut list = PlutusList::new();
		list.add(&PlutusData::new_integer(
			&(raw_scripts::ScriptId::GovernancePolicy as u32).into(),
		));
		list.add(&PlutusData::new_bytes(multi_sig_policy.script_hash().to_vec()));
		list
	}))
}

pub(crate) fn version_oracle_datum_output(
	version_oracle_validator: PlutusScript,
	version_oracle_policy: PlutusScript,
	multi_sig_policy: PlutusScript,
	network: NetworkIdKind,
	tx_context: &TransactionContext,
) -> anyhow::Result<cardano_serialization_lib::TransactionOutput> {
	let datum: PlutusData = VersionOracleDatum {
		version_oracle: raw_scripts::ScriptId::GovernancePolicy as u32,
		currency_symbol: version_oracle_policy.policy_id().0,
	}
	.into();

	let amount_builder = TransactionOutputBuilder::new()
		.with_address(&version_oracle_validator.address(network))
		.with_plutus_data(&datum)
		.with_script_ref(&ScriptRef::new_plutus_script(&multi_sig_policy.to_csl()))
		.next()?;

	let ma = MultiAsset::new()
		.with_asset_amount(&version_oracle_policy.asset(version_oracle_asset_name())?, 1u64)?;

	let output = amount_builder.with_minimum_ada_and_asset(&ma, tx_context)?.build()?;
	Ok(output)
}
