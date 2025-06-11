use serde::Deserialize;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Deserialize)]
pub struct Reserve {
	pub token_name: String,
	pub v_function_script_path: String,
	pub v_function_updated_script_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tool {
	pub cli: String,
	pub shell: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tools {
	pub cardano_cli: Tool,
	pub partner_chains_node: Tool,
	pub bech32: Tool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MainChainConfig {
	pub network: String,
	pub epoch_length: i64,
	pub slot_length: i64,
	pub active_slots_coeff: f64,
	pub security_param: i64,
	pub init_timestamp: i64,
	pub block_stability_margin: i64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct MainchainAccount {
	pub mainchain_key: String,
	pub mainchain_pub_key: Option<String>,
	pub mainchain_address: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct NodesApiConfig {
	pub governance_authority: MainchainAccount,
	pub reserve: Option<Reserve>,
	pub additional_governance_authorities: Option<Vec<MainchainAccount>>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct StackApiConfig {
	pub ogmios_scheme: Option<String>,
	pub ogmios_host: String,
	pub ogmios_port: u64,
	pub tools: Tools,
}
impl StackApiConfig {
	pub fn ogmios_url(&self) -> String {
		format!(
			"{}://{}:{}",
			self.ogmios_scheme.clone().unwrap_or("http".to_string()),
			self.ogmios_host,
			self.ogmios_port
		)
	}
}
#[derive(Debug, Clone, Deserialize)]
pub struct Timeout {
	pub long_running_function: u64,
	pub register_cmd: u64,
	pub deregister_cmd: u64,
	pub main_chain_tx: u64,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ApiConfig {
	pub genesis_utxo: String,
	pub nodes_config: NodesApiConfig,
	pub stack_config: StackApiConfig,
	pub main_chain: MainChainConfig,
	pub timeouts: Timeout,
}
impl ApiConfig {
	pub fn load() -> Self {
		let cwd = std::env::current_dir().expect("cwd exists");
		let cwd = cwd.to_str().expect("msg");
		let blockchain = "substrate";
		let nodes_env = "local";
		type JsonObj = serde_json::Map<String, JsonValue>;
		fn load_json_obj(path: &str) -> JsonObj {
			serde_json::from_str::<JsonValue>(&std::fs::read_to_string(path).unwrap())
				.unwrap()
				.as_object()
				.unwrap()
				.clone()
		}
		// It would be nicer not to copy the whole config folder but the config jsons reference the
		// location of the v-function scripts in the file system and it was not worth doing the
		// work to deal with it.
		let default_config = load_json_obj(&format!("{cwd}/config/config.json"));
		let blockchain_config_path =
			load_json_obj(&format!("{cwd}/config/{blockchain}/{nodes_env}_nodes.json"));
		let stack_config_path =
			load_json_obj(&format!("{cwd}/config/{blockchain}/{nodes_env}_stack.json"));

		fn merge(objs: &[JsonObj]) -> JsonObj {
			let mut res = JsonObj::new();
			for obj in objs {
				for (k, v) in obj {
					res.entry(k)
						.and_modify(|ev| {
							if ev.is_object() && v.is_object() {
								let ev_obj = ev.as_object().unwrap().clone();
								let v = v.as_object().unwrap().clone();
								*ev = JsonValue::Object(merge(&[ev_obj, v]));
							} else {
								*ev = v.clone();
							}
						})
						.or_insert(v.clone());
				}
			}
			res
		}
		// The original code uses Omegaconf which does a thing with merging jsons before filling up the
		// config class fields with data. I replicated this behavior here.
		let combined = merge(&[default_config, blockchain_config_path, stack_config_path]);
		serde_json::from_value::<ApiConfig>(JsonValue::Object(combined)).unwrap()
	}
}
