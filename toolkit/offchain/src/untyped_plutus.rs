#![allow(dead_code)]

use anyhow::anyhow;
use plutus::ToDatum;
use uplc::plutus_data;

pub fn datum_to_uplc_plutus_data<T: ToDatum>(datum: &T) -> uplc::PlutusData {
	plutus_data(&minicbor::to_vec(datum.to_datum()).expect("to_vec is Infallible"))
		.expect("trasformation from PC Datum to pallas PlutusData can't fail")
}

/// Map from `cardano_serialization_lib::PlutusData` to `uplc::PlutusData` via CBOR bytes.
pub fn csl_plutus_data_to_uplc(
	d: &cardano_serialization_lib::PlutusData,
) -> anyhow::Result<uplc::PlutusData> {
	let mut se = cbor_event::se::Serializer::new_vec();
	cbor_event::se::Serialize::serialize(d, &mut se).map_err(|e| anyhow!(e))?;
	let bytes = se.finalize();
	minicbor::decode(&bytes).map_err(|e| anyhow!(e.to_string()))
}

pub(crate) fn unwrap_one_layer_of_cbor(plutus_script_raw: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
	let plutus_script_bytes: uplc::PlutusData = minicbor::decode(plutus_script_raw)?;
	let plutus_script_bytes = match plutus_script_bytes {
		uplc::PlutusData::BoundedBytes(bb) => Ok(bb),
		_ => Err(anyhow!("expected validator raw to be BoundedBytes")),
	}?;
	Ok(plutus_script_bytes.into())
}
