/// Contains key definition
pub struct KeyDefinition<'a> {
	/// Display name of the key
	pub name: &'a str,
	/// Encryption scheme, one of sr25519, ed25519, or ecdsa
	pub scheme: &'a str,
	/// polkadot-sdk KeyTypeId in human readable form
	pub key_type: &'a str,
}

impl<'a> KeyDefinition<'a> {
	/// Hex encoded key type id of given key
	pub fn key_type_hex(&self) -> String {
		hex::encode(self.key_type)
	}
}

/// Well known definition of AURA key
pub const AURA: KeyDefinition<'static> =
	KeyDefinition { name: "AURA", scheme: "sr25519", key_type: "aura" };
/// Well known definition of Grandpa key
pub const GRANDPA: KeyDefinition<'static> =
	KeyDefinition { name: "Grandpa", scheme: "ed25519", key_type: "gran" };
/// Definition of Partner-chains key
pub const CROSS_CHAIN: KeyDefinition<'static> =
	KeyDefinition { name: "Cross-chain", scheme: "ecdsa", key_type: "crch" };

pub fn keystore_path(substrate_node_base_path: &str) -> String {
	format!("{substrate_node_base_path}/keystore")
}

pub fn find_existing_key(existing_keys: &[String], key_def: &KeyDefinition) -> Option<String> {
	existing_keys
		.iter()
		.find_map(|key| key.strip_prefix(&key_def.key_type_hex()))
		.map(String::from)
		.clone()
}
