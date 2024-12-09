use crate::IOContext;
use anyhow::anyhow;
use sidechain_domain::{MainchainAddressHash, MainchainPrivateKey};

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CardanoKeyFileContent {
	cbor_hex: String,
}

const CBOR_KEY_PREFIX: &str = "5820";

pub(crate) fn get_key_bytes_from_file<const N: usize>(
	path: &str,
	context: &impl IOContext,
) -> anyhow::Result<[u8; N]> {
	let Some(file_content) = context.read_file(path) else {
		return Err(anyhow!("Failed to read Cardano key file {path}"));
	};

	let json = serde_json::from_str::<CardanoKeyFileContent>(&file_content)
		.map_err(|err| anyhow!("Failed to parse Cardano key file {path}: {err:?}"))?;

	let Some(vkey) = json.cbor_hex.strip_prefix(CBOR_KEY_PREFIX) else {
		return Err(anyhow!(
			"Invalid cbor value of Cardano key - missing prefix {CBOR_KEY_PREFIX}: {}",
			json.cbor_hex
		));
	};

	let bytes: [u8; N] = hex::decode(vkey)
		.map_err(|err| {
			anyhow!("Invalid cbor value of Cardano key - not valid hex: {}\n{err:?}", json.cbor_hex)
		})?
		.try_into()
		.map_err(|_| {
			anyhow!("Invalid cbor value of Cardano key - incorrect length: {}", json.cbor_hex)
		})?;
	Ok(bytes)
}

pub(crate) fn get_mc_pkey_from_file(
	path: &str,
	context: &impl IOContext,
) -> anyhow::Result<MainchainPrivateKey> {
	Ok(MainchainPrivateKey(get_key_bytes_from_file(path, context)?))
}

pub(crate) fn get_mc_address_hash_from_pkey(pkey: &MainchainPrivateKey) -> MainchainAddressHash {
	let csl_private_key = cardano_serialization_lib::PrivateKey::from_normal_bytes(&pkey.0)
		.expect("Conversion is infallible");
	let csl_public_key_hash = csl_private_key
		.to_public()
		.hash()
		.to_bytes()
		.try_into()
		.expect("Bytes represent correct public key hash");
	MainchainAddressHash(csl_public_key_hash)
}
