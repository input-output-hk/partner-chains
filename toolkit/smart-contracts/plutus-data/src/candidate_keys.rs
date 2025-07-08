use cardano_serialization_lib::{PlutusData, PlutusList};

pub(crate) type KeyTypeId = [u8; 4];
pub(crate) type KeyTypeIdAndBytes = (KeyTypeId, Vec<u8>);

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

pub(crate) fn find_or_empty(keys: &[KeyTypeIdAndBytes], id: &KeyTypeId) -> Vec<u8> {
	keys.iter().find(|(k, _)| k == id).map(|(_, v)| v.clone()).unwrap_or_default()
}
