#![allow(dead_code)]

use anyhow::anyhow;
use pallas_primitives::conway::PlutusData;
use plutus::ToDatum;
use uplc::ast::{DeBruijn, Program};
use uplc::plutus_data;

/// This requires [`uplc`] crate and [`pallas_primitives::alonzo::PlutusData`].
/// Currently there is no other known option to apply parameters to plutus script in Rust.
///
/// Parameters:
/// * `params` - parameters to apply to the script
/// * `plutus_script_raw` - raw plutus script in CBOR format, like in `RawScripts.purs` in smart-contracts repository
pub(crate) fn apply_params_to_script<T: ToDatum>(
	params: &T,
	plutus_script_raw: &[u8],
) -> Result<Vec<u8>, anyhow::Error> {
	let params: uplc::PlutusData =
		plutus_data(&minicbor::to_vec(params.to_datum()).expect("to_vec is Infallible"))
			.expect("trasformation from PC Datum to pallas PlutusData can't fail");

	// RawScripts.purs in smart-contracts have a single layer of CBOR wrapping, so we have to unwrap it.
	let plutus_script = unwrap_one_layer_of_cbor(plutus_script_raw)?;

	let mut buffer = Vec::new();
	Program::<DeBruijn>::from_cbor(&plutus_script, &mut buffer)
		.map_err(|e| anyhow!(e.to_string()))?
		.apply_data(params)
		.to_cbor()
		.map_err(|_| anyhow!("Couldn't encode resulting script as CBOR."))
}

fn unwrap_one_layer_of_cbor(plutus_script_raw: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
	let plutus_script_bytes: PlutusData = minicbor::decode(plutus_script_raw)?;
	let plutus_script_bytes = match plutus_script_bytes {
		PlutusData::BoundedBytes(bb) => Ok(bb),
		_ => Err(anyhow!("expected validator raw to be BoundedBytes")),
	}?;
	Ok(plutus_script_bytes.into())
}

#[cfg(test)]
mod tests {
	use cardano_serialization_lib::NetworkIdKind;
	use chain_params::SidechainParams;
	use hex_literal::hex;
	use std::str::FromStr;

	#[test]
	fn apply_parameters_to_deregister() {
		// Taken from smart-contracts repository
		let candidate_validator_cbor = hex!("59013b590138010000323322323322323232322222533553353232323233012225335001100f2215333573466e3c014dd7001080909802000980798051bac330033530040022200148040dd7198011a980180311000a4010660026a600400644002900019112999ab9a33710002900009805a4810350543600133003001002300f22253350011300b49103505437002215333573466e1d20000041002133005337020089001000919199109198008018011aab9d001300735573c0026ea80044028402440204c01d2401035054350030092233335573e0024016466a0146ae84008c00cd5d100124c6010446666aae7c00480288cd4024d5d080118019aba20024988c98cd5ce00080109000891001091000980191299a800880211099a80280118020008910010910911980080200191918008009119801980100100081");

		let applied = crate::untyped_plutus::apply_params_to_script(
			&sidechain_params(),
			&candidate_validator_cbor,
		)
		.unwrap();

		// TODO: alternative is to compare to the expected CBOR.
		// It is hard to get a cbor to compare, but we can use address which is in fact returned by smart-contracts, to proved that the script is correct.
		let applied_validator_address = crate::csl::plutus_script_address(
			&applied,
			NetworkIdKind::Testnet,
			cardano_serialization_lib::LanguageKind::PlutusV2,
		)
		.to_bech32(None)
		.unwrap();

		assert_eq!(
			applied_validator_address,
			"addr_test1wq7vcwawqa29a5a2z7q8qs6k0cuvp6z2puvd8xx7vasuajq86paxz"
		);
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
			governance_authority: sidechain_domain::MainchainAddressHash(hex!(
				"76da17b2e3371ab7ca88ce0500441149f03cc5091009f99c99c080d9"
			)),
		}
	}
}
