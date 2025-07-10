//! Functions related to PlutusData decoding and encoding of candidates keys
use cardano_serialization_lib::{PlutusData, PlutusList};

pub(crate) type KeyTypeId = [u8; 4];
pub(crate) type KeyTypeIdAndBytes = (KeyTypeId, Vec<u8>);

/// Well known polkadot-sdk id of AURA key type
pub const AURA_TYPE_ID: &KeyTypeId = b"aura";
/// Well known polkadot-sdk id of Grandpa key type
pub const GRANDPA_TYPE_ID: &KeyTypeId = b"gran";
/// Key type id of Partner Chains cross-chain key, used with ECDSA cryptography
pub const CROSS_CHAIN_TYPE_ID: &KeyTypeId = b"crch";

pub(crate) fn decode_key_id_and_bytes_list(data: &PlutusData) -> Option<Vec<KeyTypeIdAndBytes>> {
	let outer_list: PlutusList = data.as_list()?;
	let mut keys: Vec<KeyTypeIdAndBytes> = Vec::with_capacity(outer_list.len());
	for kv in outer_list.into_iter() {
		keys.push(decode_key_id_and_bytes(kv)?);
	}
	Some(keys)
}

pub(crate) fn decode_key_id_and_bytes(data: &PlutusData) -> Option<KeyTypeIdAndBytes> {
	// Allow only key-value pair
	let list = data.as_list().filter(|l| l.len() == 2)?;
	let key_id: Vec<u8> = list.get(0).as_bytes()?;
	let key_id: KeyTypeId = key_id.try_into().ok()?;
	let key_bytes = list.get(1).as_bytes()?;
	Some((key_id, key_bytes))
}

pub(crate) fn key_id_and_bytes_list_to_plutus(keys: &[KeyTypeIdAndBytes]) -> PlutusData {
	let mut plutus_keys = PlutusList::new();
	for (key_type, key) in keys {
		plutus_keys.add(&key_id_and_bytes_to_plutus(&key_type, &key));
	}
	PlutusData::new_list(&plutus_keys)
}

pub(crate) fn key_id_and_bytes_to_plutus(key_type_id: &KeyTypeId, value: &[u8]) -> PlutusData {
	let mut l = PlutusList::new();
	l.add(&PlutusData::new_bytes(key_type_id.to_vec()));
	l.add(&PlutusData::new_bytes(value.to_vec()));
	PlutusData::new_list(&l)
}

/// Returns bytes of key with given id or empty vec if key wasn't found
pub fn find_or_empty(keys: &[KeyTypeIdAndBytes], id: &KeyTypeId) -> Vec<u8> {
	keys.iter().find(|(k, _)| k == id).map(|(_, v)| v.clone()).unwrap_or_default()
}
