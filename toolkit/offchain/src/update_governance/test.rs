use super::{test_values, update_governance_tx};
use crate::csl::{empty_asset_name, Costs, OgmiosUtxoExt, TransactionContext};
use crate::governance::GovernanceData;
use crate::test_values::protocol_parameters;
use cardano_serialization_lib::*;
use hex_literal::hex;
use ogmios_client::types::{Asset, Datum, OgmiosTx, OgmiosUtxo, OgmiosValue};
use pretty_assertions::assert_eq;
use sidechain_domain::MainchainKeyHash;

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
			id: hex!("1bc6eeebd308616860384b9748801d586a93a7291faedb464e73e9f6355e392b"),
		},
		index: 0,
		value: OgmiosValue { lovelace: 9922945937, native_tokens: [].into() },
		address: test_address_bech32(),

		..OgmiosUtxo::default()
	}
}

fn version_oracle_validator_hash() -> [u8; 28] {
	hex!("c11dee532646a9b226aac75f77ea7ae5fba9270674327c882794701e")
}

fn version_oracle_token_name() -> Vec<u8> {
	hex!("56657273696f6e206f7261636c65").to_vec()
}

fn governance_script_id() -> BigInt {
	32u64.into()
}

fn governance_utxo() -> OgmiosUtxo {
	OgmiosUtxo {
		transaction: OgmiosTx {
			id: hex!("40db7e41a67c5c560aa3d4bce389cb2eecd7c5f88188dbe472eb95069d1357b3"),
		},
		index: 0,
		value: OgmiosValue {
			lovelace: 2945937,
			native_tokens: [(
				version_oracle_validator_hash(),
				vec![Asset { name: version_oracle_token_name(), amount: 1 }],
			)]
			.into(),
		},
		address: "addr_test1wqrlc9gqxnyyzwyzgtvrf77famec87zme6zfxgq2sq4up8gccxfnc".to_string(),
		datum: Some(Datum { bytes: version_oracle_validator_hash().to_vec() }),
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
	crate::plutus_script::PlutusScript { language: Language::new_plutus_v2(), bytes: vec![] }
}

fn governance_data() -> GovernanceData {
	GovernanceData { policy_script: governance_script(), utxo: governance_utxo() }
}

fn new_governance_authority() -> MainchainKeyHash {
	MainchainKeyHash(hex_literal::hex!("84ba05c28879b299a8377e62128adc7a0e0df3ac438ff95efc7c8443"))
}

fn mint_ex_units() -> ExUnits {
	ExUnits::new(&333u64.into(), &555u64.into())
}

fn spend_ex_units() -> ExUnits {
	ExUnits::new(&111u64.into(), &222u64.into())
}

fn test_costs() -> Costs {
	Costs::new(
		vec![(governance_script().csl_script_hash(), mint_ex_units())]
			.into_iter()
			.collect(),
		vec![(0, spend_ex_units())].into_iter().collect(),
	)
}

fn multisig_policy_hash() -> [u8; 28] {
	// important: this is the hash of the multisig policy parametrized with the *old* authority
	hex!("a646474b8f5431261506b6c273d307c7569a4eb6c96b42dd4a29520a")
}

fn version_oracle_validator_address() -> Address {
	Address::from_bech32("addr_test1wqrlc9gqxnyyzwyzgtvrf77famec87zme6zfxgq2sq4up8gccxfnc").unwrap()
}

fn test_update_governance_tx() -> Transaction {
	update_governance_tx(
		test_values::MULTI_SIG_POLICY,
		test_values::VERSION_ORACLE_VALIDATOR,
		test_values::VERSION_ORACLE_POLICY,
		genesis_utxo().to_domain(),
		new_governance_authority(),
		&governance_data(),
		test_costs(),
		&tx_context(),
	)
	.expect("Test transaction should be constructed without error")
}

#[test]
fn update_governance_test() {
	let tx: serde_json::Value =
		serde_json::from_str(&test_update_governance_tx().to_json().unwrap()).unwrap();

	assert_eq!(tx, test_values::test_update_governance_tx())
}

#[test]
fn mints_a_token_using_multisig_policy() {
	let multisig_token_minted_amount = (test_update_governance_tx().body().mint())
		.expect("Should mint a token")
		.get(&multisig_policy_hash().into())
		.and_then(|policy| policy.get(0))
		.expect("The minted token should have the multi-sig policy")
		.get(&empty_asset_name())
		.expect("The minted token should have an empty asset name");

	assert_eq!(multisig_token_minted_amount, Int::new_i32(1))
}

#[test]
fn output_contains_version_oracle_plutus_data() {
	let outputs = test_update_governance_tx().body().outputs();

	let script_output = (outputs.into_iter())
		.find(|o| o.address() == version_oracle_validator_address())
		.expect("Should create a utxo at version oracle validator address");

	let plutus_data = script_output.plutus_data().expect("Utxo should have plutus data attached");

	assert_eq!(
		plutus_data,
		PlutusData::new_list(&{
			let mut list = PlutusList::new();
			list.add(&PlutusData::new_integer(&governance_script_id()));
			list.add(&PlutusData::new_bytes(version_oracle_validator_hash().to_vec()));
			list
		})
	);
}

#[test]
fn consumes_the_previous_governance_utxo() {
	let inputs = test_update_governance_tx().body().inputs();

	let input_utxos: Vec<_> = inputs.into_iter().map(|input| input.to_string()).collect();

	assert!(input_utxos.contains(&governance_utxo().to_string()))
}

#[test]
fn contains_correct_redeemers() {
	let redeemers = test_update_governance_tx().witness_set().redeemers().unwrap();

	assert_eq!(redeemers.len(), 2);

	let spend_redeemer = redeemers.get(0);
	assert_eq!(spend_redeemer.tag(), RedeemerTag::new_spend());
	assert_eq!(spend_redeemer.index(), 1u64.into());
	assert_eq!(spend_redeemer.data(), PlutusData::new_integer(&32u64.into()));
	assert_eq!(spend_redeemer.ex_units(), spend_ex_units());

	let mint_redeemer = redeemers.get(1);
	assert_eq!(mint_redeemer.tag(), RedeemerTag::new_mint());
	assert_eq!(mint_redeemer.index(), 0u64.into());
	assert_eq!(mint_redeemer.data(), PlutusData::new_empty_constr_plutus_data(&0u64.into()));
	assert_eq!(mint_redeemer.ex_units(), mint_ex_units());
}
