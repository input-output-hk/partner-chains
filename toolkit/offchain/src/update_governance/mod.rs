#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use crate::{
	await_tx::{self, AwaitTx},
	csl::{
		convert_ex_units, convert_value, empty_asset_name, InputsBuilderExt, OgmiosUtxoExt,
		TransactionBuilderExt, TransactionContext,
	},
	init_governance::{self, transaction::version_oracle_datum_output, GovernanceData},
	plutus_script::PlutusScript,
	scripts_data::{multisig_governance_policy_configuration, version_scripts_and_address},
};
use anyhow::{anyhow, Context};
use cardano_serialization_lib::{
	Coin, DatumSource, ExUnits, Int, LanguageKind, MintBuilder, MintWitness, MultiAsset,
	PlutusData, PlutusScriptSource, PlutusWitness, Redeemer, RedeemerTag, Transaction,
	TransactionBuilder, TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::{OgmiosTx, OgmiosUtxo},
};
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use sidechain_domain::{
	byte_string::ByteString, MainchainAddressHash, MainchainPrivateKey, McTxHash, UtxoId, UtxoIndex,
};

#[cfg(test)]
mod test_values;

pub async fn run_update_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	new_governance_authority: MainchainAddressHash,
	payment_key: MainchainPrivateKey,
	genesis_utxo_id: UtxoId,
	client: &T,
	await_tx: A,
) -> anyhow::Result<OgmiosTx> {
	let tx_context = TransactionContext::for_payment_key(payment_key.0, client).await?;
	let (version_validator, version_policy, version_validator_address) =
		version_scripts_and_address(genesis_utxo_id, tx_context.network)?;

	log::info!(
		"Querying version oracle validator address ({version_validator_address}) for utxos..."
	);
	let version_utxos = client.query_utxos(&[version_validator_address.clone()]).await?;

	let governance_data = init_governance::get_governance_data(genesis_utxo_id, client).await?;

	let tx = update_governance_tx(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		genesis_utxo_id,
		new_governance_authority,
		&tx_context,
		&governance_data,
		ExUnits::new(&0u64.into(), &0u64.into()),
		ExUnits::new(&0u64.into(), &0u64.into()),
	)?;

	let costs = client.evaluate_transaction(&tx.to_bytes()).await?;
	if costs.len() != 2 {
		return Err(anyhow!("Error retrieving witness costs: expected 2 entries."));
	};

	let Some(mint_cost) = costs.iter().find(|cost| cost.validator.purpose == "mint") else {
		return Err(anyhow!("Error retrieving witness costs: mint cost data missing."));
	};
	let Some(spend_cost) = costs.iter().find(|cost| cost.validator.purpose == "spend") else {
		return Err(anyhow!("Error retrieving witness costs: spend cost data missing."));
	};

	let tx = update_governance_tx(
		raw_scripts::MULTI_SIG_POLICY,
		raw_scripts::VERSION_ORACLE_VALIDATOR,
		raw_scripts::VERSION_ORACLE_POLICY,
		genesis_utxo_id,
		new_governance_authority,
		&tx_context,
		&governance_data,
		convert_ex_units(&mint_cost.budget),
		convert_ex_units(&spend_cost.budget),
	)?;
	let signed_tx = tx_context.sign(&tx);

	let response = client.submit_transaction(&signed_tx.to_bytes()).await?;
	println!("Submitted transaction: {}", hex::encode(response.transaction.id));

	await_tx
		.await_tx_output(
			client,
			UtxoId { tx_hash: McTxHash(response.transaction.id), index: UtxoIndex(0) },
		)
		.await?;

	Ok(response.transaction)
}

fn update_governance_tx(
	multi_sig_policy: &[u8],
	version_oracle_validator: &[u8],
	version_oracle_policy: &[u8],
	genesis_utxo: UtxoId,
	new_governance_authority: MainchainAddressHash,
	tx_context: &TransactionContext,
	governance_data: &GovernanceData,
	mint_ex_units: ExUnits,
	spend_ex_units: ExUnits,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		PlutusScript::from_wrapped_cbor(multi_sig_policy, LanguageKind::PlutusV2)?
			.apply_uplc_data(multisig_governance_policy_configuration(new_governance_authority))?;
	let version_oracle_validator =
		PlutusScript::from_wrapped_cbor(version_oracle_validator, LanguageKind::PlutusV2)?
			.apply_data(genesis_utxo)?;
	let version_oracle_policy =
		PlutusScript::from_wrapped_cbor(version_oracle_policy, LanguageKind::PlutusV2)?
			.apply_data(genesis_utxo)?
			.apply_uplc_data(version_oracle_validator.address_data(tx_context.network)?)?;

	let config = crate::csl::get_builder_config(tx_context)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	tx_builder.add_mint_one_script_token_using_reference_script(
		&multi_sig_policy,
		&governance_data.utxo_id_as_tx_input(),
		&mint_ex_units,
	)?;

	tx_builder.add_output(&version_oracle_datum_output(
		version_oracle_validator.clone(),
		version_oracle_policy.clone(),
		multi_sig_policy.clone(),
		tx_context.network,
		tx_context,
	)?)?;

	tx_builder.set_inputs(&{
		let mut inputs = TxInputsBuilder::new();
		inputs.add_script_utxo_input_with_data(
			&governance_data.utxo,
			&version_oracle_validator,
			&PlutusData::new_integer(&32u32.into()),
			&spend_ex_units,
		)?;

		inputs
	});

	Ok(tx_builder.balance_update_and_build(tx_context)?)
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::test_values::protocol_parameters;
	use cardano_serialization_lib::*;
	use hex_literal::hex;
	use ogmios_client::types::{Asset, Datum, OgmiosValue};
	use pretty_assertions::assert_eq;

	fn payment_key_domain() -> MainchainPrivateKey {
		MainchainPrivateKey(hex!(
			"94f7531c9639654b77fa7e10650702b6937e05cd868f419f54bcb8368e413f04"
		))
	}

	fn payment_key() -> PrivateKey {
		PrivateKey::from_normal_bytes(&payment_key_domain().0).unwrap()
	}

	fn test_address_bech32() -> String {
		"addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw".into()
	}

	fn payment_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("1bc6eeebd308616860384b9748801d586a93a7291faedb464e73e9f6355e392b"),
			},
			index: 0,
			value: OgmiosValue { lovelace: 9922945937, native_tokens: [].into() },
			address: test_address_bech32(),

			..OgmiosUtxo::default()
		}
	}

	fn governance_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("40db7e41a67c5c560aa3d4bce389cb2eecd7c5f88188dbe472eb95069d1357b3"),
			},
			index: 0,
			value: OgmiosValue {
				lovelace: 2945937,
				// native_tokens: [].into(),
				native_tokens: [(
					hex!("c11dee532646a9b226aac75f77ea7ae5fba9270674327c882794701e"),
					vec![Asset { name: hex!("56657273696f6e206f7261636c65").to_vec(), amount: 1 }],
				)]
				.into(),
			},
			address: "addr_test1wqrlc9gqxnyyzwyzgtvrf77famec87zme6zfxgq2sq4up8gccxfnc".to_string(),
			datum: Some(Datum {
				bytes: hex!("9f1820581cc11dee532646a9b226aac75f77ea7ae5fba9270674327c882794701eff")
					.to_vec(),
			}),
			..OgmiosUtxo::default()
		}
	}

	fn tx_context() -> TransactionContext {
		TransactionContext {
			payment_key: payment_key(),
			payment_key_utxos: vec![payment_utxo()],
			network: NetworkIdKind::Testnet,
			protocol_parameters: protocol_parameters(),
		}
	}

	fn genesis_utxo() -> OgmiosUtxo {
		OgmiosUtxo {
			transaction: OgmiosTx {
				id: hex!("071ce86f4b21214f35df5e7f2931a10b67f4a11360e56c1e2bcd7978980adca5"),
			},
			index: 1,
			value: OgmiosValue::new_lovelace(10000),
			address: test_address_bech32(),

			..Default::default()
		}
	}

	fn governance_script() -> crate::plutus_script::PlutusScript {
		crate::plutus_script::PlutusScript { language: LanguageKind::PlutusV2, bytes: vec![] }
	}

	fn governance_data() -> GovernanceData {
		GovernanceData {
			policy_script: governance_script(),
			utxo_id: governance_utxo().to_domain(),
			utxo: governance_utxo(),
		}
	}

	#[test]
	fn update_governance_test() {
		let tx: serde_json::Value = serde_json::from_str(
			&update_governance_tx(
				test_values::MULTI_SIG_POLICY,
				test_values::VERSION_ORACLE_VALIDATOR,
				test_values::VERSION_ORACLE_POLICY,
				genesis_utxo().to_domain(),
				MainchainAddressHash(hex_literal::hex!(
					"76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
				)),
				&tx_context(),
				&governance_data(),
				ExUnits::new(&0u64.into(), &0u64.into()),
				ExUnits::new(&0u64.into(), &0u64.into()),
			)
			.unwrap()
			.to_json()
			.unwrap(),
		)
		.unwrap();

		assert_eq!(tx, test_values::test_update_governance_tx())
	}
}
