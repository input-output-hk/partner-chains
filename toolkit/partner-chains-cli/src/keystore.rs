pub struct KeyDefinition<'a> {
	pub name: &'a str,
	pub scheme: &'a str,
	pub key_type: &'a str,
}

impl<'a> KeyDefinition<'a> {
	pub fn key_type_hex(&self) -> String {
		hex::encode(self.key_type)
	}
}

pub const AURA: KeyDefinition<'static> =
	KeyDefinition { name: "Aura", scheme: "sr25519", key_type: "aura" };
pub const GRANDPA: KeyDefinition<'static> =
	KeyDefinition { name: "Grandpa", scheme: "ed25519", key_type: "gran" };
pub const IM_ONLINE: KeyDefinition<'static> =
	KeyDefinition { name: "ImOnline", scheme: "sr25519", key_type: "imon" };
pub const CROSS_CHAIN: KeyDefinition<'static> =
	KeyDefinition { name: "Cross-chain", scheme: "ecdsa", key_type: "crch" };

pub fn keystore_path(substrate_node_base_path: &str, chain_name: &str) -> String {
	format!("{substrate_node_base_path}/chains/{chain_name}/keystore")
}

pub fn find_existing_key(existing_keys: &[String], key_def: &KeyDefinition) -> Option<String> {
	existing_keys
		.iter()
		.find_map(|key| key.strip_prefix(&key_def.key_type_hex()))
		.map(String::from)
		.clone()
}
