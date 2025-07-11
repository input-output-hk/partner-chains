//! Functions related to PlutusData decoding and encoding of candidates keys
use cardano_serialization_lib::{PlutusData, PlutusList};
use sidechain_domain::{CandidateKey, CandidateKeys};

pub(crate) fn decode_candidate_keys(data: &PlutusData) -> Option<CandidateKeys> {
	let outer_list: PlutusList = data.as_list()?;
	let mut keys: Vec<CandidateKey> = Vec::with_capacity(outer_list.len());
	for kv in outer_list.into_iter() {
		keys.push(decode_candidate_key(kv)?);
	}
	Some(CandidateKeys(keys))
}

pub(crate) fn decode_candidate_key(data: &PlutusData) -> Option<CandidateKey> {
	// Allow only key-value pair
	let list = data.as_list().filter(|l| l.len() == 2)?;
	let key_id: Vec<u8> = list.get(0).as_bytes()?;
	let key_id: [u8; 4] = key_id.try_into().ok()?;
	let key_bytes = list.get(1).as_bytes()?;
	Some(CandidateKey { id: key_id, bytes: key_bytes })
}

pub(crate) fn candidate_keys_to_plutus(keys: &CandidateKeys) -> PlutusData {
	let mut plutus_keys = PlutusList::new();
	for key in keys.0.iter() {
		plutus_keys.add(&candidate_key_to_plutus(key));
	}
	PlutusData::new_list(&plutus_keys)
}

pub(crate) fn candidate_key_to_plutus(key: &CandidateKey) -> PlutusData {
	let mut l = PlutusList::new();
	l.add(&PlutusData::new_bytes(key.id.to_vec()));
	l.add(&PlutusData::new_bytes(key.bytes.clone()));
	PlutusData::new_list(&l)
}
