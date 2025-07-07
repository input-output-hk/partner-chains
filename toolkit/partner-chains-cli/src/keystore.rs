/// Human readable definition of keys
pub struct KeyDefinition<'a> {
	/// Human readable name of the key, for displaying to the user
	pub name: &'a str,
	/// Identifier of encryption scheme
	pub scheme: &'a str,
	/// Polkadot-sdk compatible KeyTypeId
	pub key_type: &'a str,
}

impl<'a> KeyDefinition<'a> {
	/// Encodes key type as hex.
	pub fn key_type_hex(&self) -> String {
		hex::encode(self.key_type)
	}
}

/// Data of AURA key
pub const AURA: KeyDefinition<'static> =
	KeyDefinition { name: "Aura", scheme: "sr25519", key_type: "aura" };
/// Data of Grandpa key
pub const GRANDPA: KeyDefinition<'static> =
	KeyDefinition { name: "Grandpa", scheme: "ed25519", key_type: "gran" };
/// Data of PC cross-chain key
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
