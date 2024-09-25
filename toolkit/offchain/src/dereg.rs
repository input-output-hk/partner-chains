use crate::ogmios::{self, OgmiosUtxo};
use crate::{plutus_data, tx};
use anyhow::anyhow;
use cardano_serialization_lib::ExUnits;
use chain_params::SidechainParams;
use db_sync_follower::candidates::RegisterValidatorDatum;
use hex_literal::hex;
use jsonrpsee::http_client::HttpClient;
use pallas_addresses::{ShelleyAddress, ShelleyDelegationPart, ShelleyPaymentPart};
use pallas_primitives::alonzo::PlutusData;
use plutus::ToDatum;
use sidechain_domain::{MainchainAddressHash, MainchainPublicKey};
use std::str::FromStr;
use uplc::{
	ast::{DeBruijn, Program},
	plutus_data,
};

/// This is copied from smart-contracts repo
pub const CANDIDATE_VALIDATOR_CBOR: [u8; 318] = hex!("59013b590138010000323322323322323232322222533553353232323233012225335001100f2215333573466e3c014dd7001080909802000980798051bac330033530040022200148040dd7198011a980180311000a4010660026a600400644002900019112999ab9a33710002900009805a4810350543600133003001002300f22253350011300b49103505437002215333573466e1d20000041002133005337020089001000919199109198008018011aab9d001300735573c0026ea80044028402440204c01d2401035054350030092233335573e0024016466a0146ae84008c00cd5d100124c6010446666aae7c00480288cd4024d5d080118019aba20024988c98cd5ce00080109000891001091000980191299a800880211099a80280118020008910010910911980080200191918008009119801980100100081");

pub async fn deregister(network: pallas_addresses::Network) -> anyhow::Result<()> {
	let spo_pub_key = MainchainPublicKey(hex!(
		"3c765f942325121842ec9ab25f66077b73411db98e55a72f6135747b14840fed"
	));
	let own_payment_vkey = hex!("a35ef86f1622172816bb9e916aea86903b2c8d32c728ad5c9b9472be7e3c5e88");
	let own_payment_key_hash = sidechain_domain::crypto::blake2b(&own_payment_vkey);
	// addr_test1vqezxrh24ts0775hulcg3ejcwj7hns8792vnn8met6z9gwsxt87zy
	let own_addr = ShelleyAddress::new(
		network,
		ShelleyPaymentPart::key_hash(own_payment_key_hash.into()),
		ShelleyDelegationPart::Null,
	);
	println!("own_addr: {:?}", own_addr.to_bech32().unwrap());

	let applied_validator = apply_params_to_script(&sidechain_params(), &CANDIDATE_VALIDATOR_CBOR)?;
	let validator_address = plutus_script_v2_address(applied_validator.clone(), network);
	println!("validator_address: {:?}", validator_address.to_bech32().unwrap());

	let client = HttpClient::builder().build("http://localhost:1337")?;
	let own_utxos = ogmios::query_utxos(&own_addr, &client).await?;
	let validator_utxos = ogmios::query_utxos(&validator_address, &client).await?;
	println!("There are {} validator utxos", validator_utxos.len());
	println!("There are {} own utxos", own_utxos.len());
	let own_registration_utxos =
		find_own_registration_utxos(&own_payment_key_hash, &spo_pub_key, validator_utxos);
	println!("There are {} own registration utxos", own_registration_utxos.len());

	let protocol_parameters = ogmios::query_protocol_parameters(&client).await?;
	let payment_skey = hex!("cf86dc85e4933424826e846c18d2695689bf65de1fc0c40fcd9389ba1cbdc069");

	let tx_with_invalid_budget = tx::make_deregister_tx(
		own_utxos.first().unwrap(),
		&own_registration_utxos,
		&own_addr,
		own_payment_key_hash,
		protocol_parameters.clone(),
		applied_validator.clone(),
		ExUnits::new(&0u32.into(), &0u32.into()),
	)
	.map_err(|e| anyhow!("Error when building de-register transaction: {}", e))?;

	let costs = ogmios::evalutate_tx(&tx_with_invalid_budget.to_bytes(), &client).await?;
	let cost = costs.first().unwrap();

	println!("cost: {:#?}", cost);
	let unsigned_tx = tx::make_deregister_tx(
		&own_utxos.first().unwrap(),
		&own_registration_utxos,
		&own_addr,
		own_payment_key_hash,
		protocol_parameters.clone(),
		applied_validator.clone(),
		tx::convert_ex_units(&cost.budget),
	)
	.unwrap();

	let signed_tx = tx::sign_tx(&unsigned_tx, &payment_skey);
	//println!("{}", tx_bytes.clone());
	let res = ogmios::submit_tx(&signed_tx.to_bytes(), &client).await?;
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
			let d = utxo.datum.clone();
			let rd_opt: Option<RegisterValidatorDatum> = d
				.and_then(|datum| hex::decode(datum).ok())
				.and_then(|cbor_bytes| minicbor::decode(&cbor_bytes).ok())
				.and_then(|pd: PlutusData| plutus_data::decode_register_validator_datum(&pd));
			if let Some(rd) = rd_opt {
				&rd.own_pkh == own_pkh && &rd.stake_ownership.pub_key == spo_pub_key
			} else {
				false
			}
		})
		.collect()
}

fn plutus_script_v2_address(
	script_bytes: Vec<u8>,
	network: pallas_addresses::Network,
) -> ShelleyAddress {
	// Before hashing the script, we need to prepend with byte 0x02, because this is PlutusV2 script
	let mut buf: Vec<u8> = vec![2];
	buf.extend(script_bytes);
	ShelleyAddress::new(
		network,
		ShelleyPaymentPart::script_hash(sidechain_domain::crypto::blake2b(buf.as_slice()).into()),
		ShelleyDelegationPart::Null,
	)
}

fn apply_params_to_script(
	params: &SidechainParams,
	plutus_script_bytes_wrapped: &[u8],
) -> Result<Vec<u8>, anyhow::Error> {
	let params_cbor =
		minicbor::to_vec(params.clone().to_datum()).expect("Datum is encodable to cbor");
	let params_in_pallas = plutus_data(&params_cbor)
		.expect("trasformation from PC Datum to pallas PlutusData can't fail");

	let plutus_script = unwrap_one_layer_of_cbor(plutus_script_bytes_wrapped)?;

	let mut buffer = Vec::new();
	let mut program = Program::<DeBruijn>::from_cbor(&plutus_script, &mut buffer)
		.map_err(|e| anyhow!(e.to_string()))?;
	program = program.apply_data(params_in_pallas);

	match program.to_cbor() {
		Ok(res) => Ok(res),
		Err(_) => Err(anyhow!("Couldn't encode resulting script as CBOR.".to_string())),
	}
}

/// RawScripts.purs in smart-contracts have a single layer of CBOR wrapping, so we have to unwrap it.
fn unwrap_one_layer_of_cbor(plutus_script_bytes_wrapped: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
	let plutus_script_bytes: PlutusData = minicbor::decode(&plutus_script_bytes_wrapped).unwrap();
	let plutus_script_bytes = match plutus_script_bytes {
		PlutusData::BoundedBytes(bb) => Ok(bb),
		_ => Err(anyhow!("expected validator raw to be BoundedBytes")),
	}?;
	Ok(plutus_script_bytes.into())
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
