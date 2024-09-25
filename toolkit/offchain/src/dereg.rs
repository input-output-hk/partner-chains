#![allow(dead_code)]

use crate::tx;
use anyhow::anyhow;
use cardano_serialization_lib::{
	Credential, EnterpriseAddress, ExUnits, LanguageKind, NetworkIdKind, PlutusData,
};
use chain_params::SidechainParams;
use db_sync_follower::candidates::RegisterValidatorDatum;
use hex_literal::hex;
use jsonrpsee::http_client::HttpClient;
use ogmios_client::{
	query_ledger_state::QueryLedgerState, transactions::Transactions, types::OgmiosUtxo,
};
use sidechain_domain::{MainchainAddressHash, MainchainPublicKey};
use std::str::FromStr;
/// This is copied from smart-contracts repo
pub const CANDIDATE_VALIDATOR_CBOR: [u8; 318] = hex!("59013b590138010000323322323322323232322222533553353232323233012225335001100f2215333573466e3c014dd7001080909802000980798051bac330033530040022200148040dd7198011a980180311000a4010660026a600400644002900019112999ab9a33710002900009805a4810350543600133003001002300f22253350011300b49103505437002215333573466e1d20000041002133005337020089001000919199109198008018011aab9d001300735573c0026ea80044028402440204c01d2401035054350030092233335573e0024016466a0146ae84008c00cd5d100124c6010446666aae7c00480288cd4024d5d080118019aba20024988c98cd5ce00080109000891001091000980191299a800880211099a80280118020008910010910911980080200191918008009119801980100100081");

pub async fn deregister(network: pallas_addresses::Network) -> anyhow::Result<()> {
	let spo_pub_key = MainchainPublicKey(hex!(
		"3c765f942325121842ec9ab25f66077b73411db98e55a72f6135747b14840fed"
	));
	let own_payment_vkey = hex!("a35ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be7e3c5e88");
	let own_payment_key_hash = sidechain_domain::crypto::blake2b(&own_payment_vkey);
	// addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy
	let own_addr = EnterpriseAddress::new(
		network.value(),
		&Credential::from_keyhash(&own_payment_key_hash.into()),
	)
	.to_address();
	println!("own_addr: {:?}", own_addr.to_bech32(None).unwrap());

	let applied_validator = crate::untyped_plutus::apply_params_to_script(
		&sidechain_params(),
		&CANDIDATE_VALIDATOR_CBOR,
	)?;
	//validator_address: "addr_test1wq7vcwawqa29a5a2z7q8qs6k0cuvp6z2puvd8xx7vasuajq86paxz"
	let validator_address = crate::csl::plutus_script_address(
		&applied_validator,
		NetworkIdKind::Testnet,
		LanguageKind::PlutusV2,
	);
	println!("validator_address: {:?}", validator_address.to_bech32(None)?);

	let client = HttpClient::builder().build("http://localhost:1337")?;
	let own_utxos = client.query_utxos(&[own_addr.to_bech32(None)?]).await?;
	let validator_utxos = client.query_utxos(&[validator_address.to_bech32(None)?]).await?;
	println!("There are {} validator utxos", validator_utxos.len());
	println!("There are {} own utxos", own_utxos.len());
	let own_registration_utxos =
		find_own_registration_utxos(&own_payment_key_hash, &spo_pub_key, validator_utxos);
	println!("There are {} own registration utxos", own_registration_utxos.len());

	let protocol_parameters = client.query_protocol_parameters().await?;
	let payment_skey = hex!("cf86dc85e4933424826e846c18d2695689bf65de1fc0c40fcd9389ba1cbdc069");

	let tx_with_invalid_budget = tx::make_deregister_tx(
		own_utxos.first().unwrap(),
		&own_registration_utxos,
		&own_addr,
		own_payment_key_hash,
		&protocol_parameters,
		applied_validator.clone(),
		ExUnits::new(&0u32.into(), &0u32.into()),
	)
	.map_err(|e| anyhow!("Error when building de-register transaction: {}", e))?;

	let costs = client.evaluate_transaction(&tx_with_invalid_budget.to_bytes()).await?;
	let cost = costs.first().unwrap();

	println!("cost: {:#?}", cost);
	let unsigned_tx = tx::make_deregister_tx(
		own_utxos.first().unwrap(),
		&own_registration_utxos,
		&own_addr,
		own_payment_key_hash,
		&protocol_parameters,
		applied_validator.clone(),
		crate::csl::convert_ex_units(&cost.budget),
	)
	.unwrap();

	let signed_tx = tx::sign_tx(&unsigned_tx, &payment_skey);
	//println!("{}", tx_bytes.clone());
	let res = client.submit_transaction(&signed_tx.to_bytes()).await?;
	println!("{:#?}", res);
	Ok(())
}

fn find_own_registration_utxos(
	own_pkh: &[u8; 28],
	spo_pub_key: &MainchainPublicKey,
	validator_utxos: Vec<OgmiosUtxo>,
) -> Vec<OgmiosUtxo> {
	validator_utxos
		.into_iter()
		.filter(|utxo| {
			let datum = utxo.datum.clone();
			let rd_opt: Option<RegisterValidatorDatum> = datum
				.and_then(|d| PlutusData::from_bytes(d.bytes).ok())
				.and_then(|pd: PlutusData| {
					db_sync_follower::candidates::datum::decode_legacy_register_validator_datum(pd)
				});
			match rd_opt {
				Some(RegisterValidatorDatum::V0 {
					stake_ownership,
					own_pkh: datum_own_pkh,
					..
				}) => &datum_own_pkh == own_pkh && &stake_ownership.pub_key == spo_pub_key,
				_ => false,
			}
		})
		.collect()
}

fn sidechain_params() -> SidechainParams {
	SidechainParams {
		chain_id: 111,
		threshold_numerator: 2,
		threshold_denominator: 3,
		genesis_committee_utxo: sidechain_domain::UtxoId::from_str(
			"0000000000000000000000000000000000000000000000000000000000000000#0",
		)
		.unwrap(),
		governance_authority: MainchainAddressHash(hex!(
			"76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
		)),
	}
}
