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
					"address": "addr_test1wz6yy027g25x2z23f7aa4sjlpcyzdypglt9u5g8wg3punqqavqkqu",
					"amount": {
						"coin": "1430920",
						"multiasset": {
							"ebd6732339bd12eafa0941d7c1a7efcf442ef5107d402a16ce25898c": {
								"56657273696f6e206f7261636c65": "1"
							}
						}
					},
					"plutus_data": {
						"Data": "{\"list\":[{\"int\":32},{\"bytes\":\"ebd6732339bd12eafa0941d7c1a7efcf442ef5107d402a16ce25898c\"}]}"
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
						"coin": "9921134930",
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
			"fee": "390087",
			"ttl": null,
			"certs": null,
			"withdrawals": null,
			"update": null,
			"auxiliary_data_hash": null,
			"validity_start_interval": null,
			"mint": [
				[
					"ebd6732339bd12eafa0941d7c1a7efcf442ef5107d402a16ce25898c",
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
					"coin": "9922360806",
					"multiasset": {
						"01010101010101010101010101010101010101010101010101010101": {
							"": "1"
						}
					}
				},
				"plutus_data": null,
				"script_ref": null
			},
			"total_collateral": "585131",
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
				"590cbf0100003332323233223232323232323233223232323232323232323233223232323232323232232323232322225335323232323233353232325333573466e1d200000213322122233002005004375a6ae84004dd71aba1357440022a666ae68cdc3a40040042664424446600200a0086eb4d5d08009bae357426ae8800454ccd5cd19b87480100084c84888c00c010dd69aba1001130314901035054310035573c0046aae74004dd50039191919299a998082481174552524f522d56455253494f4e2d504f4c4943592d3037003303422533500110332213235003223500122225333500210072153500522350172233532335005233500425333573466e3c0080045400c40b880b88cd401080b894ccd5cd19b8f00200115003102e153350032153350022133500223350022335002233500223303000200120312335002203123303000200122203122233500420312225333573466e1c01800c54ccd5cd19b8700500213302c00400110331033102c153350012102c102c133044225335001100e22132533500321350012253353302c00201c153353302c333027010502048810e56657273696f6e206f7261636c6500480084cd41200d0010401040104004c010004c8cd4104004108c094014403084020c010004c080c07cc04cdd619801180091000a40002a66a660229201174552524f522d56455253494f4e2d504f4c4943592d3038005335330342253350011033221325333573466e24ccc050c078cc01cd4c0c800c880052002500d4890e56657273696f6e206f7261636c65004800040044cd40d40e0004c010004c0a4c04cdd619801180091000a4008203844203a266022921174552524f522d56455253494f4e2d504f4c4943592d3039003300e500800a101b101b5302c002502a50042232533533010491174552524f522d56455253494f4e2d504f4c4943592d303100300c302a302930123758660026a6058660026a605801244002900011000a40002a66a6601e9201174552524f522d56455253494f4e2d504f4c4943592d30320033005003002133010491174552524f522d56455253494f4e2d504f4c4943592d3033005004101a101a502a2253353300e491174552524f522d56455253494f4e2d504f4c4943592d30340033004002001153353300f491174552524f522d56455253494f4e2d504f4c4943592d3035003300c500600813300f4901174552524f522d56455253494f4e2d504f4c4943592d30360050031019101913300f33300a30143233502835302900122001480214009400d2210e56657273696f6e206f7261636c65004800888cc0c0894cd400440bc884c8d400c88894ccd40084014854cd4008854d401888d404888cd4c8cd40148cd401094ccd5cd19b8f00200115003102920292335004202925333573466e3c0080045400c40a454cd400c854cd400884cd40088cd40088cd40088cd40088cc0ac00800480b08cd400880b08cc0ac0080048880b0888cd401080b08894ccd5cd19b8700600315333573466e1c0140084cc09c01000440b840b8409c54cd40048409c409c4cc0fc894cd40044034884c94cd400c84d4004894cd4cc09c00806454cd4cc0b403406054cd4cc09cccc088045406d2210e56657273696f6e206f7261636c6500480084cd410c0bc0104010401040104004c010004c8cd40f00040f4c080018402c401884018c010004c068c8c068c040dd619a8149a981500091000a4008a006266a04a6a604c0064400290000a99aa99a9a981299a8121a981280111000a400444a66a0022a042442a66a0022a666ae68cdc3a4000008260480042a046442a04a4260426eb80045407c840044c0a92401164552524f522d4f5241434c452d504f4c4943592d313000301a003102a1302949010350543500302722533500110102215333573466ebc024008404c4c01000488c8c8c94cd4c94cd4c8cc0b4894cd400454088884d4008894ccd5cd19b8f002007130270011300600300253353302c225335001102b2213235003223500122225333500210072153350022133039225335001100b2213253350032135001225335330210024810054ccd5cd19b8748008ccc07003406d2210e56657273696f6e206f7261636c6500133503d009004100410041001300400132335036001037301a0021008210083004001300e3232323302f225335001100322133502f00230040010023012300d37586600c6004440029000180818061bac33005300122001480094c094cc010cc098800400920001302a491194552524f522d56455253494f4e2d43555252454e43592d30310022153350011002221302e491194552524f522d56455253494f4e2d43555252454e43592d3031002130210011501f3233301e75ca03a002660066a6048660066604a4002002900011000a401042a66a00220264426a00444a66a0062666ae68cdc4800a400002e03044203220246aae78004dd50012810111191981491299a8008a40004426a00444a666ae68cdc78010048980380089803001802181411299a8008a40004426a00444a666ae68cdc780100388008980300191299a80089812001110a99a8008801110981400311299a8008806899ab9c00200c30212233335573e0024042466a0406ae84008c00cd5d100124c44666ae68cdc380100080500491999999aba4001250142501423232333002375800800244a6646aa66a6666660020064400c400a400a46036002400a42603600220084266600c00600a44a66a666666008004440124010401040104603c00242666012004603c246600200a00444014200e4444446666666ae900188c8cc01cd55ce8009aab9e001375400e4600a6eac01c8c010dd6003918019bad00723002375c00e0542006a02a4446666aae7c00c800c8cc008d5d08021aba2004023250142501401f301e225335001101d22133501e300f0023004001301d225335001101c22133501d0023004001301c225335001101b22133501c0023004001233300e75ca01a00244666ae68cdc7801000802001891001091000980b91299a800880b11099a80b8011802000980b11299a800880a91099a80b18040011802000980a91299a800880a11099a80a8011802000980a11299a800880991099a80a1802801180200091919192999ab9a3370e90000010999109198008018011919192999ab9a3370e90000010999109198008018011919192999ab9a3370e900000109bae35742002260369201035054310035573c0046aae74004dd51aba1001375a6ae84d5d10008980c2481035054310035573c0046aae74004dd51aba10013005357426ae880044c055241035054310035573c0046aae74004dd500091919192999ab9a3370e9000001099191999911109199980080280200180118039aba100333300a75ca0126ae84008c8c8c94ccd5cd19b87480000084488800c54ccd5cd19b87480080084c84888c004010dd71aba100115333573466e1d20040021321222300200435742002260329201035054310035573c0046aae74004dd51aba10013300875c6ae84d5d10009aba200135744002260289201035054310035573c0046aae74004dd500091919192999ab9a3370e90000010991991091980080180118009aba10023300623232325333573466e1d200000213212230020033005357420022a666ae68cdc3a400400426466644424466600200a0080066eb4d5d08011bad357420026eb4d5d09aba200135744002260309201035054310035573c0046aae74004dd50009aba1357440044646464a666ae68cdc3a400000426424460040066eb8d5d08008a999ab9a3370e900100109909118008019bae357420022602e921035054310035573c0046aae74004dd500089809a49035054310035573c0046aae74004dd5000911919192999ab9a3370e90010010a8040a999ab9a3370e90000010980498029aba1001130134901035054310035573c0046aae74004dd5000899800bae75a4464460046eac004c04088cccd55cf800900811919a8081980798031aab9d001300535573c00260086ae8800cd5d0801008909118010018891000980591299a800880511099a8058011802000980511299a800880491099a8050011802000980491299a800880411099a80499a8029a980300111000a4000600800226444a666ae68cdc4000a4000260129210350543600133003001002300822253350011300949103505437002215333573466e1d20000041002133005337020089001000919198021aab9d0013233004200100135573c0026ea80048c88c008004c01c88cccd55cf8009003919a80318021aba10023003357440049311091980080180109100109109119800802001919319ab9c001002120012323001001223300330020020014c012bd8799fd8799f5820992a24e743a522eb3adf0bc39820a9a52093525f91ed6205b72fd4087c13b4acff01ff004c0129d8799fd87a9f581cb4423d5e42a86509514fbbdac25f0e08269028facbca20ee4443c980ffd87a80ff0001"
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

	assert_eq!(transaction, expected_transaction())
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
