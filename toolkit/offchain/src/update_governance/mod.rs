#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
use crate::{
	await_tx::AwaitTx,
	csl::{InputsBuilderExt, OgmiosUtxoExt, TransactionBuilderExt, TransactionContext},
	init_governance::transaction::{
		multisig_governance_policy_configuration, version_oracle_datum_output,
	},
	plutus_script::PlutusScript,
};
use cardano_serialization_lib::{
	Coin, ExUnits, LanguageKind, PlutusData, Transaction, TransactionBuilder,
	TransactionOutputBuilder, TxInputsBuilder,
};
use ogmios_client::{
	query_ledger_state::{QueryLedgerState, QueryUtxoByUtxoId},
	query_network::QueryNetwork,
	transactions::Transactions,
	types::{OgmiosTx, OgmiosUtxo},
};
use partner_chains_plutus_data::version_oracle::VersionOracleDatum;
use sidechain_domain::{MainchainAddressHash, MainchainPrivateKey, UtxoId};

#[cfg(test)]
mod test_values;

pub async fn run_update_governance<
	T: QueryLedgerState + Transactions + QueryNetwork + QueryUtxoByUtxoId,
	A: AwaitTx,
>(
	_new_governance_authority: MainchainAddressHash,
	_payment_key: MainchainPrivateKey,
	_genesis_utxo_id: UtxoId,
	_client: &T,
	_await_tx: A,
) -> anyhow::Result<OgmiosTx> {
	// 1. find the utxo with the {VersionOracle, 32} datum under the VersionOracleValidator address
	//    (it also contains the 1 governance token)
	Ok(OgmiosTx::default())
}

fn update_governance_tx(
	multi_sig_policy: &[u8],
	version_oracle_validator: &[u8],
	version_oracle_policy: &[u8],
	genesis_utxo: OgmiosUtxo,
	governance_utxo: OgmiosUtxo,
	new_governance_authority: MainchainAddressHash,
	tx_context: &TransactionContext,
	mint_ex_units: ExUnits,
) -> anyhow::Result<Transaction> {
	let multi_sig_policy =
		PlutusScript::from_wrapped_cbor(multi_sig_policy, LanguageKind::PlutusV2)?
			.apply_uplc_data(multisig_governance_policy_configuration(new_governance_authority))?;
	let version_oracle_validator =
		PlutusScript::from_wrapped_cbor(version_oracle_validator, LanguageKind::PlutusV2)?
			.apply_uplc_data(genesis_utxo.to_uplc_plutus_data())?;
	let version_oracle_policy =
		PlutusScript::from_wrapped_cbor(version_oracle_policy, LanguageKind::PlutusV2)?
			.apply_uplc_data(genesis_utxo.to_uplc_plutus_data())?
			.apply_uplc_data(version_oracle_validator.address_data(tx_context.network)?)?;

	let config = crate::csl::get_builder_config(tx_context)?;
	let mut tx_builder = TransactionBuilder::new(&config);

	// tx_builder.add_mint_one_script_token(&version_oracle_policy, mint_ex_units)?;

	// tx_builder.add_output(&version_oracle_datum_output(
	// 	version_oracle_validator.clone(),
	// 	version_oracle_policy.clone(),
	// 	multi_sig_policy,
	// 	tx_context.network,
	// 	tx_context,
	// )?)?;

	// tx_builder.add_output(&{
	// 	TransactionOutputBuilder::new()
	// 		.with_address(&version_oracle_validator.address(tx_context.network))
	// 		.with_plutus_data(
	// 			&VersionOracleDatum {
	// 				version_oracle: 32,
	// 				currency_symbol: version_oracle_policy.script_hash(),
	// 			}
	// 			.into(),
	// 		)
	// 		.next()?
	// 		.with_coin(coin)
	// 		.build()?
	// })?;

	// tx_builder.add_output_with_one_script_token(
	// 	&version_oracle_validator,
	// 	&version_oracle_policy,
	// 	&VersionOracleDatum {
	// 		version_oracle: 32,
	// 		currency_symbol: version_oracle_policy.script_hash(),
	// 	}
	// 	.into(),
	// 	&tx_context,
	// )?;

	tx_builder.set_inputs(&{
		TxInputsBuilder::with_key_inputs(&[governance_utxo], &tx_context.payment_key_hash())?
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

	#[test]
	fn update_governance_test() {
		let tx: serde_json::Value = serde_json::from_str(
			&update_governance_tx(
				test_values::MULTI_SIG_POLICY,
				test_values::VERSION_ORACLE_VALIDATOR,
				test_values::VERSION_ORACLE_POLICY,
				genesis_utxo(),
				governance_utxo(),
				MainchainAddressHash::default(),
				&tx_context(),
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
