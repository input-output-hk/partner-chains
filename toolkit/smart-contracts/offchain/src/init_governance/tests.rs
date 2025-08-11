use super::transaction::*;
use crate::await_tx::mock::ImmediateSuccess;
use crate::cardano_keys::CardanoPaymentSigningKey;
use crate::csl::Costs;
use crate::governance::MultiSigParameters;
use crate::init_governance::run_init_governance;
use crate::scripts_data;
use crate::test_values::{ogmios_native_1_of_1_script, ogmios_plutus_script, protocol_parameters};
use crate::{csl::TransactionContext, ogmios_mock::MockOgmiosClient};
use cardano_serialization_lib::{Address, ExUnits, NetworkIdKind};
use hex_literal::*;
use ogmios_client::transactions::{
	OgmiosBudget, OgmiosEvaluateTransactionResponse, OgmiosValidatorIndex,
	SubmitTransactionResponse,
};
use ogmios_client::types::*;
use pretty_assertions::assert_eq;
use serde_json::json;
use sidechain_domain::MainchainKeyHash;

fn expected_transaction() -> serde_json::Value {
	json!({
		"body": {
			"inputs": [
				{
					"transaction_id": "23249849e52ee17143509baf7a5abcbd76f9b589947d73e7bbc03ca9142f9535",
					"index": 1
				},
				{
					"transaction_id": "992a24e743a522eb3adf0bc39820a9a52093525f91ed6205b72fd4087c13b4ac",
					"index": 1
				}
			],
			"outputs": [
				{
					"address": "addr_test1wrs65fxjz2s3qzt24s98g99v7z94y9khymaqpqmw6fe8p2s35kcac",
					"amount": {
						"coin": "1430920",
						"multiasset": {
							"b11fe70530cfb701c8f3a2776cddeb0019aa456eba7d0b56d548b341": {
								"56657273696f6e206f7261636c65": "1"
							}
						}
					},
					"plutus_data": {
						"Data": "{\"list\":[{\"int\":32},{\"bytes\":\"b11fe70530cfb701c8f3a2776cddeb0019aa456eba7d0b56d548b341\"}]}"
					},
					"script_ref": {
						"NativeScript": {
							"ScriptNOfK": {
								"n": 1,
								"native_scripts": [
									{
										"ScriptPubkey": {
											"addr_keyhash": "76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
										}
									}
								]
							}
						}
					}
				},
				{
					"address": "addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw",
					"amount": {
						"coin": "9921163222",
						"multiasset": {
							"01010101010101010101010101010101010101010101010101010101": {
								"": "1"
							}
						}
					},
					"plutus_data": null,
					"script_ref": null
				}
			],
			"fee": "361795",
			"ttl": null,
			"certs": null,
			"withdrawals": null,
			"update": null,
			"auxiliary_data_hash": null,
			"validity_start_interval": null,
			"mint": [
				[
					"b11fe70530cfb701c8f3a2776cddeb0019aa456eba7d0b56d548b341",
					{
						"56657273696f6e206f7261636c65": "1"
					}
				]
			],
			"script_data_hash": "b916952fd30d8123e403045126a3aae97a45c70bb7bfe31e34994a2e6e05a28f",
			"collateral": [
				{
					"transaction_id": "23249849e52ee17143509baf7a5abcbd76f9b589947d73e7bbc03ca9142f9535",
					"index": 1
				}
			],
			"required_signers": null,
			"network_id": null,
			"collateral_return": {
				"address": "addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw",
				"amount": {
					"coin": "9922403244",
					"multiasset": {
						"01010101010101010101010101010101010101010101010101010101": {
							"": "1"
						}
					}
				},
				"plutus_data": null,
				"script_ref": null
			},
			"total_collateral": "542693",
			"reference_inputs": null,
			"required_signers": [
				"76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
			],
			"voting_procedures": null,
			"voting_proposals": null,
			"donation": null,
			"current_treasury_value": null
		},
		"witness_set": {
			"vkeys": null,
			"native_scripts": null,
			"bootstraps": null,
			"plutus_scripts": [
				"590a3c010000333322323233223232323232323322232323232323232323232323232323232322225335323232325335333573466e1d200035573a00204e04c2646464666a00e464a66a6602a4c664600200244a66a00220444426466602c6ae84d5d11aab9e3754006444466605a0044a66a6464a66a6605a6ae84008d5d0800899299a98009aba13574400642a66a60046ae84d5d1001909929919a999ab9a3370e90001aab9d00203d03c1325335333573466e1d200035573a00207c07a2660666ae84d55cf0019aba135573c00226004931baa003130014988ccc0cc010888ccc0d80188894cd4ccd5cd19b8700600304304215335333573466e1c01400810c1084ccd5cd19b87004001043042104210422040203d375400420722a66a60026ae84d5d10011081c881c980f017881b9aab9e375402e6aae78dd5002899199180080091299a800880491099299a801909a80091299a9981e80100a8a99a9981e99981300680c2450e56657273696f6e206f7261636c6500480084cc0240fc010401040104004cc010010004cc004c0c00080ad40a4401880188008cc010010004c004ccc06801c06005c54cd54cd54cd4cc8c004004894cd40044088884c94cd4ccd5cd19b8933301837566ae84d5d11aab9e375400601491010e56657273696f6e206f7261636c6500480000c00c44cd40900ac0044004cc010010004c004c06401c40b08840b440b040ac54cd4cc03801402840b040ac40ac40acc054088894cd54cd4c034ccc06801c06005c40b040ac54cd4cc05498cc01000800454cd5400c40b040ac40ac40ac894cd4cc05498cc01000800454cd54cd4cc03801402840b040ac54cd5400c40b040ac40ac40ac4cc09cccc040ccc05c0100340300092210e56657273696f6e206f7261636c65004800888cc8c004004894cd40044080884c8ccc05000c8888ccc0ac00894cd4008854cd4c8c94cd4cc0b0d5d08011aba100113253353001357426ae8800c854cd4c008d5d09aba20032132532335333573466e1d200035573a004078076264a66a666ae68cdc3a40006aae740040f40f04cc0c8d5d09aab9e003357426aae780044c0092637540062600293119981900211119981a803111299a999ab9a3370e00c0060840822a66a666ae68cdc38028010210208999ab9a3370e00800208408220822082407e40786ea800840e054cd4c004d5d09aba2002210381038301d02e103635573c6ea8058d55cf1baa0061323323001001225335001100a22132533500321350012253353303c00201615335333573466e3c0280540f80f454cd4cc0f0ccc09403805d22010e56657273696f6e206f7261636c6500480084cc0240f80104010401040104004cc010010004cc004c0bc00c0a940a0401c401880188008cc010010004cc050084c05c014dd71aba135573c0022c6ea8d5d09aba2002357420026aae78dd5001191919299a999ab9a3370e9000001013012899910911198010028021bad357420026eb8d5d09aba200115335333573466e1d200200202602513322122233001005004375a6ae84004dd71aba1357440022a66a666ae68cdc3a400800404c04a264244460060086eb4d5d08008b1aab9e00235573a0026ea8008526163230010012233301e020225335325335333573466e3cc04c058c04c00409008c4ccd5cd19b87301501630150010240231023357426aae78dd500108118998020020008009119299a99299aa99a998051ba95335332300100122533500110182213233300c357426ae88d55cf1baa0032222325335333573466e1d200435573a0020580562a66a0044264664600200244a66a002201444264a66a006426a00244a66a6606800490200a99a999ab9a3370e900119980e80700c2450e56657273696f6e206f7261636c650003603513300900a00410041004100133004004001330013027357426aae7800c089408040184018dd5001100119802002000999180080091981000b9119a80c001198018018009991800800919810198018078071119aba0002330030030013300222222222222200b261622153350011002221600121301f37560022a03a42603e66460020024660420304466a032664424660020060046eb8d55ce8011bad35573c004660060060020022a03a6600200a00842a66a00220424426a00444a66a0062666ae68cdc4800a400004a04c44204e2040601c6ae84d55cf1baa001262222222222220082223232323333005357420066eacd5d08011aba10013300775c6ae84d5d10009aba2001357440026aae78dd500191191919299a999ab9a3370e900100100f80f0a80d0a99a999ab9a3370e900000100f80f0980d98029aba10011635573c0046aae74004dd500091119a998021ba900200323233533006375200600246eb400520003756002900011199180080091980c280b1299a999ab9a3375e0086aae7400407006c48c064d55cf001091980180180080091299a80089801009910a99a8008801110980300b919180080091980a8061119a8069802001198018018009311111111111100611998010009111111111110051311119191919191919191919191999999999998069bac357420166eb0d5d08051bac357420126eacd5d08041bab3574200e6eb0d5d08031bab3574200a6ae84010dd61aba100337566ae84008dd59aba10013010357426ae88004d5d10009aba2001357440026ae88004d5d10009aba2001357440026ae88004d5d10009aba200135573c6ea800c8c008d5d09aab9e37540024646464a66a666ae68cdc3a400000402802626eb8d5d08008b1aab9e00235573a0026ea80048dd69aba1357446aae78dd500089100109109119800802001911929919a999ab9a3370e90001aab9d00200f00e1325335333573466e1d200035573a00202001e2666ae68cdc79bae357426aae7800cdd71aba135573c00202001e26004931baa0031300149894cd4ccd5cd19b8748008d55ce801007807099299a999ab9a3370e90011aab9d00101000f1333573466e3cdd71aba135573c0066eb8d5d09aab9e00101000f100f3754006201c6ea8008888c94cd4ccd5cd19b8748008d55ce800807006899191998029bad357420046eb4d5d08009bad357426ae88004d5d10009aab9e00113002498dd5001900091119299a999ab9a3370e90021aab9d00100c00b13003357426aae780044c00926375400646666666ae9000494010940108c8cccc01c940188894cc8d4cccccc00489402c94028940288488c00800c9402801084cccc0309402c8894cd4cccccc01489403c9403894038940388488c00800c00c84cccc0408488c00848cc004024014889404400c034540340100248888894cccccd5d2000899198039aab9d00135573c0026ea80044c014dd5800898021bac00113003375a002260046eb800454020004010dd60011280212802001090009091180100188910009112999aab9f0011003133002357420026ae8800488ccd5cd19b870020010040031220021220014c12bd8799fd8799f5820992a24e743a522eb3adf0bc39820a9a52093525f91ed6205b72fd4087c13b4acff01ff004c0129d8799fd87a9f581ce1aa24d212a110096aac0a7414acf08b5216d726fa00836ed27270aaffd87a80ff0001"
			],
			"plutus_data": null,
			"redeemers": [
				{
					"tag": "Mint",
					"index": "0",
					"data": "{\"constructor\":0,\"fields\":[{\"int\":32},{\"bytes\":\"fe9046c83d510b767477b9f7f4817d546da295d112bb2dedc213f6cd\"}]}",
					"ex_units": {
						"mem": "789754",
						"steps": "171220003"
					}
				}
			]
		},
		"is_valid": true,
		"auxiliary_data": null
	})
}

#[test]
fn transaction_creation() {
	let transaction: serde_json::Value = serde_json::from_str(
		&init_governance_transaction(
			&MultiSigParameters::new_one_of_one(&governance_authority()),
			genesis_utxo(),
			test_costs(),
			&tx_context(),
		)
		.unwrap()
		.to_json()
		.unwrap(),
	)
	.unwrap();

	assert_eq!(expected_transaction(), transaction)
}

#[test]
fn plutus_script_attached_to_genesis_utxo_increases_fee() {
	let fee_when_genesis_utxo_has_no_script = &init_governance_transaction(
		&MultiSigParameters::new_one_of_one(&governance_authority()),
		genesis_utxo(),
		test_costs(),
		&tx_context(),
	)
	.unwrap()
	.body()
	.fee();

	let fee_when_genesis_utxo_has_plutus_script = &init_governance_transaction(
		&MultiSigParameters::new_one_of_one(&governance_authority()),
		genesis_utxo_with_plutus_script(),
		test_costs(),
		&tx_context(),
	)
	.unwrap()
	.body()
	.fee();
	assert!(fee_when_genesis_utxo_has_no_script < fee_when_genesis_utxo_has_plutus_script);
}

#[test]
fn native_attached_to_genesis_utxo_increases_fee() {
	let fee_when_genesis_utxo_has_no_script = &init_governance_transaction(
		&MultiSigParameters::new_one_of_one(&governance_authority()),
		genesis_utxo(),
		test_costs(),
		&tx_context(),
	)
	.unwrap()
	.body()
	.fee();

	let fee_when_genesis_utxo_has_native_script = &init_governance_transaction(
		&MultiSigParameters::new_one_of_one(&governance_authority()),
		genesis_utxo_with_native_script(),
		test_costs(),
		&tx_context(),
	)
	.unwrap()
	.body()
	.fee();
	assert!(fee_when_genesis_utxo_has_no_script < fee_when_genesis_utxo_has_native_script);
}

#[tokio::test]
async fn transaction_run() {
	let transaction_id = [2; 32];
	let transaction = OgmiosTx { id: transaction_id };
	let budget = OgmiosBudget { memory: 100, cpu: 200 };
	let mock_client = MockOgmiosClient::new()
		.with_protocol_parameters(protocol_parameters())
		.with_utxos(vec![genesis_utxo(), payment_utxo()])
		.with_evaluate_result(vec![OgmiosEvaluateTransactionResponse {
			budget,
			validator: OgmiosValidatorIndex { index: 0, purpose: "mint".to_owned() },
		}])
		.with_submit_result(SubmitTransactionResponse { transaction });

	let genesis_utxo = genesis_utxo().utxo_id();
	let result = run_init_governance(
		&MultiSigParameters::new_one_of_one(&governance_authority()),
		&payment_key(),
		Some(genesis_utxo),
		&mock_client,
		ImmediateSuccess,
	)
	.await
	.expect("Should succeed");

	assert_eq!(result.tx_hash.0, transaction_id);
	assert_eq!(result.genesis_utxo, genesis_utxo);
}

fn genesis_utxo() -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx {
			id: hex!("992a24e743a522eb3adf0bc39820a9a52093525f91ed6205b72fd4087c13b4ac"),
		},
		index: 1,
		value: OgmiosValue::new_lovelace(10000),
		address: test_address_bech32(),

		..Default::default()
	}
}

fn genesis_utxo_with_plutus_script() -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx {
			id: hex!("992a24e743a522eb3adf0bc39820a9a52093525f91ed6205b72fd4087c13b4ac"),
		},
		index: 1,
		value: OgmiosValue::new_lovelace(10000),
		address: test_address_bech32(),
		script: Some(ogmios_plutus_script()),
		..Default::default()
	}
}

fn genesis_utxo_with_native_script() -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx {
			id: hex!("992a24e743a522eb3adf0bc39820a9a52093525f91ed6205b72fd4087c13b4ac"),
		},
		index: 1,
		value: OgmiosValue::new_lovelace(10000),
		address: test_address_bech32(),
		script: Some(ogmios_native_1_of_1_script()),
		..Default::default()
	}
}

const PAYMENT_KEY_BYTES: [u8; 32] =
	hex!("94f7531c9639654b77fa7e10650702b6937e05cd868f419f54bcb8368e413f04");

fn payment_key() -> CardanoPaymentSigningKey {
	CardanoPaymentSigningKey::from_normal_bytes(PAYMENT_KEY_BYTES).unwrap()
}

fn payment_address() -> Address {
	Address::from_bech32("addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw").unwrap()
}

fn tx_context() -> TransactionContext {
	TransactionContext {
		payment_key: payment_key(),
		payment_key_utxos: vec![payment_utxo()],
		network: NetworkIdKind::Testnet,
		protocol_parameters: protocol_parameters(),
		change_address: payment_address(),
	}
}

fn test_costs() -> Costs {
	Costs::new(
		vec![(
			version_oracle_policy().csl_script_hash(),
			ExUnits::new(&789754u64.into(), &171220003u64.into()),
		)]
		.into_iter()
		.collect(),
		vec![].into_iter().collect(),
	)
}

fn version_oracle_policy() -> crate::plutus_script::PlutusScript {
	scripts_data::version_oracle(genesis_utxo().utxo_id(), tx_context().network)
		.unwrap()
		.policy
}

fn governance_authority() -> MainchainKeyHash {
	MainchainKeyHash(hex!("76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"))
}

fn payment_utxo() -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx {
			id: hex!("23249849e52ee17143509baf7a5abcbd76f9b589947d73e7bbc03ca9142f9535"),
		},
		index: 1,
		value: OgmiosValue {
			lovelace: 9922945937,
			native_tokens: [([1; 28], vec![Asset { name: vec![], amount: 1 }])].into(),
		},
		address: test_address_bech32(),

		..OgmiosUtxo::default()
	}
}

fn test_address_bech32() -> String {
	"addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw".into()
}
