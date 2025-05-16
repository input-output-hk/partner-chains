use cardano_serialization_lib::*;
use hex_literal::hex;
use pallas_primitives::MaybeIndefArray;
use partner_chains_cardano_offchain::plutus_script;
use partner_chains_cardano_offchain::scripts_data::version_oracle;
use sidechain_domain::UtxoId;

/// Builds a transaction that sets up the chain governance policy using Partner Chains Plutus Smart Contract.
/// The script is applied with the chain genesis utxo and the governance authority key hash.
/// This is needed to prove that the offchain code keep compatibility with the legacy chain governance policy.
pub fn legacy_governance_init_transaction(
	genesis_utxo: UtxoId,
	governance_authority_key: [u8; 32],
) -> Vec<u8> {
	let multisig_policy_script = {
		let governance_authority_key_hash =
			PrivateKey::from_normal_bytes(&governance_authority_key)
				.unwrap()
				.to_public()
				.hash()
				.to_bytes();
		use uplc::PlutusData::{Array, BigInt, BoundedBytes};
		plutus_script![
			raw_scripts::MULTI_SIG_POLICY,
			Array(MaybeIndefArray::Indef(vec![
				Array(MaybeIndefArray::Indef(vec![BoundedBytes(
					governance_authority_key_hash.into()
				),])),
				BigInt(uplc::BigInt::Int(1.into())),
			]))
		]
		.unwrap()
		.to_csl()
	};

	let version_oracle_data = version_oracle(genesis_utxo, NetworkIdKind::Testnet).unwrap();
	let version_oracle_plutus_script = version_oracle_data.policy.to_csl();
	let version_oracle_script_hash: PolicyID = version_oracle_plutus_script.hash().into();
	let governance_policy_asset_name =
		AssetName::new(hex!("56657273696f6e206f7261636c65").to_vec()).unwrap();

	let mut outputs = TransactionOutputs::new();

	outputs.add(&{
		let mut assets = Assets::new();
		assets.insert(&governance_policy_asset_name, &1u32.into());
		let mut gov_token_ma = MultiAsset::new();
		gov_token_ma.insert(&version_oracle_script_hash, &assets);
		let mut output = TransactionOutput::new(
			&Address::from_bech32(
				"addr_test1wplvesjjxtg8lhyy34ak2dr9l3kz8ged3hajvcvpanfx7rcwzvtc5",
			)
			.unwrap(),
			&Value::new_with_assets(&999500000u32.into(), &gov_token_ma),
		);
		output.set_script_ref(&ScriptRef::new_plutus_script(&multisig_policy_script));

		let mut output_plutus_data = PlutusList::new();
		output_plutus_data.add(&PlutusData::new_integer(&32u32.into()));
		output_plutus_data.add(&PlutusData::new_bytes(version_oracle_script_hash.to_bytes()));

		output.set_plutus_data(&PlutusData::new_list(&output_plutus_data));
		output
	});

	let mut inputs = TransactionInputs::new();
	inputs.add(&TransactionInput::new(&genesis_utxo.tx_hash.0.into(), genesis_utxo.index.0.into()));

	let mut body = TransactionBody::new_tx_body(&inputs, &outputs, &500000u32.into());
	body.set_mint(&Mint::new_from_entry(
		&version_oracle_script_hash,
		&MintAssets::new_from_entry(&governance_policy_asset_name, &Int::new_i32(1)).unwrap(),
	));
	body.set_collateral(&inputs);

	// This should not change. If transaction submission fails on this script hash,
	// it means that some policy has changed and the previous version of it should be
	// copied to this tests instead of being used by `raw_scripts` reference.
	body.set_script_data_hash(
		&hex!("bb4035b9ede213192640b6e68ddea7d6c42ad664a9b4d1fbff04b52193cec1ae").into(),
	);

	let mut witness_set = TransactionWitnessSet::new();
	witness_set.set_plutus_scripts(&{
		let mut plutus_scripts = PlutusScripts::new();
		plutus_scripts.add(&version_oracle_plutus_script);
		plutus_scripts
	});
	witness_set.set_redeemers(&{
		let mut redeemers = Redeemers::new();
		let mut redeemer_plutus_list = PlutusList::new();
		redeemer_plutus_list.add(&PlutusData::new_integer(&32u32.into()));
		redeemer_plutus_list.add(&PlutusData::new_bytes(multisig_policy_script.hash().to_bytes()));
		redeemers.add(&Redeemer::new(
			&RedeemerTag::new_mint(),
			&0u32.into(),
			&PlutusData::new_constr_plutus_data(&ConstrPlutusData::new(
				&0u32.into(),
				&redeemer_plutus_list,
			)),
			&ExUnits::new(&789754u32.into(), &229713721u32.into()),
		));
		redeemers
	});
	witness_set.set_vkeys(&{
		let mut vkeys = Vkeywitnesses::new();
		vkeys.add(&{
			let tx_hash: [u8; 32] = sidechain_domain::crypto::blake2b(body.to_bytes().as_ref());
			let private_key = PrivateKey::from_normal_bytes(&governance_authority_key).unwrap();
			let signature = private_key.sign(&tx_hash);
			Vkeywitness::new(&Vkey::new(&private_key.to_public()), &signature)
		});
		vkeys
	});

	let tx = Transaction::new(&body, &witness_set, Option::None);
	tx.to_bytes()
}
