use crate::{
	apiconfig::ApiConfig,
	blockchain_api::{CardanoCli, PartnerChainsNode, SubstrateApi},
	conftest::*,
};
use serde_json::Value as JsonValue;

const INITIAL_RESERVE_DEPOSIT: i64 = 1000;

const MIN_LOVELACE_FOR_TX: i64 = 20_000_000;
const MIN_LOVELACE_TO_COVER_FEES: i64 = 10_000_000;

#[derive(Debug)]
struct VFunction {
	cbor: String,
	script_path: String,
	script_hash: String,
	address: String,
	reference_utxo: String,
}

struct Reserve {
	token: String,
	v_function: VFunction,
}

fn reserve_asset_id(config: &ApiConfig, api: &SubstrateApi) -> Result<String, String> {
	let asset_name = config.nodes_config.reserve.as_ref().unwrap().token_name.clone();
	let asset_name_hex = hex::encode(asset_name.as_bytes());
	let policy_id = minting_policy_id(api, config)?;
	Ok(format!("{}.{}", policy_id, asset_name_hex))
}

fn transaction_input(config: &ApiConfig, api: &SubstrateApi) -> Result<Option<String>, String> {
	let utxo_dict = api.cardano_cli().get_utxos(&governance_address(config))?;
	Ok(utxo_dict
		.iter()
		.find(|utxo| utxo.1["value"]["lovelace"].as_i64().expect("i64") > MIN_LOVELACE_FOR_TX)
		.map(|x| x.0.clone()))
}

fn payment_key(config: &ApiConfig) -> String {
	config.nodes_config.governance_authority.mainchain_key.clone()
}

fn cbor_to_bech32(cbor: &str, prefix: &str) -> Result<String, String> {
	let d = &hex::decode(cbor).map_err(|e| e.to_string())?[2..];
	let hrp = bech32::Hrp::parse(prefix).map_err(|e| e.to_string())?;
	bech32::encode::<bech32::Bech32>(hrp, d).map_err(|e| e.to_string())
}

fn hex_to_bech32(hex_string: &str, prefix: &str) -> Result<String, String> {
	let d = hex::decode(hex_string.strip_prefix("0x").unwrap_or(hex_string))
		.map_err(|e| e.to_string())?;
	let hrp = bech32::Hrp::parse(prefix).map_err(|e| e.to_string())?;
	bech32::encode::<bech32::Bech32>(hrp, &d).map_err(|e| e.to_string())
}

fn v_function_address(api: &SubstrateApi) -> Result<String, String> {
	let verification_key = api.cardano_cli().generate_payment_keys()?.1;
	let bech32_vkey = cbor_to_bech32(verification_key["cborHex"].as_str().unwrap(), "addr_vk")?;
	api.cardano_cli().build_address(&bech32_vkey)
}
fn read_v_function_script_file(script_path: String) -> Result<JsonValue, String> {
	let script_string = std::fs::read_to_string(script_path).map_err(|e| e.to_string())?;
	serde_json::from_str::<JsonValue>(&script_string).map_err(|e| e.to_string())
}
fn attach_v_function_to_utxo(
	api: &SubstrateApi,
	config: &ApiConfig,
	address: &str,
	filepath: &str,
) -> Result<String, String> {
	log::info!("Attaching V-function to {address}...");
	let lovelace_amount = MIN_LOVELACE_FOR_TX - MIN_LOVELACE_TO_COVER_FEES;
	let raw_tx_filepath = api
		.cardano_cli()
		.build_tx_with_reference_script(
			&transaction_input(config, api)?.expect("utxo exists"),
			&address,
			lovelace_amount,
			&filepath,
			&governance_address(config),
		)?
		.1;

	let signed_tx_filepath =
		api.cardano_cli().sign_transaction(&raw_tx_filepath, &payment_key(config))?;

	api.cardano_cli().submit_transaction(&signed_tx_filepath)
}

fn v_function(api: &SubstrateApi, config: &ApiConfig) -> Result<VFunction, String> {
	let v_function_path =
		config.nodes_config.reserve.as_ref().unwrap().v_function_script_path.clone();
	log::info!("Creating V-function from {v_function_path}...");
	let v_function_script = read_v_function_script_file(v_function_path)?;
	let v_function_cbor = v_function_script["cborHex"].as_str().expect("string");
	let script_path = write_file(
		&api.cardano_cli().run_command,
		&v_function_script.to_string().replace("\"", "\\\""), // TODO unacceptable
	)?;
	let script_hash = api.cardano_cli().get_policy_id(&script_path)?;
	let v_function_address = v_function_address(api)?;
	attach_v_function_to_utxo(api, config, &v_function_address, &script_path)?;
	let reference_utxo = wait_until(
		"reference utxo is observable",
		|| {
			let utxo_dict = api.cardano_cli().get_utxos(&v_function_address).ok()?;
			utxo_dict
				.iter()
				.find(|utxo| utxo.1["referenceScript"]["script"]["cborHex"] == v_function_cbor)
				.map(|utxo| utxo.0.clone())
		},
		config.timeouts.main_chain_tx,
		3,
	)?;
	let v_function = VFunction {
		cbor: v_function_cbor.to_string(),
		script_path,
		script_hash,
		address: v_function_address,
		reference_utxo,
	};
	log::info!("V-function successfully created: {v_function:?}");
	Ok(v_function)
}

fn reserve(
	api: &SubstrateApi,
	config: &ApiConfig,
	v_function: VFunction,
) -> Result<Reserve, String> {
	Ok(Reserve { token: reserve_asset_id(config, api)?, v_function })
}

fn create_reserve(config: &ApiConfig, api: &SubstrateApi) -> Result<(), String> {
	let reserve = reserve(api, &config, v_function(api, &config)?)?;
	api.partner_chains_node().reserve_create(
		&reserve.v_function.script_hash,
		INITIAL_RESERVE_DEPOSIT,
		&reserve.token,
		&payment_key(config),
	)?;
	log::info!("Reserve created with initial deposit of {INITIAL_RESERVE_DEPOSIT} tokens");
	Ok(())
}

fn clean_up_reserve(config: &ApiConfig, api: &SubstrateApi) -> Result<JsonValue, String> {
	log::info!("Cleaning up reserve (handover)...");
	let payment_key = &payment_key(config);
	api.partner_chains_node().reserve_handover(payment_key)
}

fn governance_address(config: &ApiConfig) -> String {
	config.nodes_config.governance_authority.mainchain_address.clone()
}

fn governance_vkey_bech32(config: &ApiConfig) -> Result<String, String> {
	let vkey = &config.nodes_config.governance_authority.mainchain_pub_key.clone().unwrap();
	hex_to_bech32(vkey, "addr_vk")
}

fn minting_policy_filepath(
	api: &SubstrateApi,
	config: &ApiConfig,
) -> Result<std::string::String, std::string::String> {
	let key_hash = api.cardano_cli().get_address_key_hash(&governance_vkey_bech32(config)?)?;
	let policy_script = format!(r##"{{\"keyHash\": \"{key_hash}\", \"type\": \"sig\"}}"##);
	write_file(&api.cardano_cli().run_command, &policy_script)
}

fn minting_policy_id(api: &SubstrateApi, config: &ApiConfig) -> Result<String, String> {
	let minting_policy_filepath = &minting_policy_filepath(api, config)?;
	api.cardano_cli().get_policy_id(minting_policy_filepath)
}

fn reserve_initial_balance(api: &SubstrateApi, config: &ApiConfig) -> Result<i64, String> {
	api.get_mc_balance(
		addresses(api)["ReserveValidator"].as_str().expect("str"),
		&reserve_asset_id(&config, api)?,
	)
}

fn native_token_balance(api: &SubstrateApi, config: &ApiConfig) -> Result<i64, String> {
	let balance =
		api.get_mc_balance(&governance_address(&config), &reserve_asset_id(&config, api)?)?;
	log::info!("Native token balance: {balance}");
	Ok(balance)
}

fn amount_to_deposit(api: &SubstrateApi, config: &ApiConfig) -> Result<i64, String> {
	let reserve_initial_balance = reserve_initial_balance(api, &config)?;
	Ok(rand::random_range(1i64..100i64.min(reserve_initial_balance)))
}

fn deposit_funds(
	api: &SubstrateApi,
	config: &ApiConfig,
	amount_to_deposit: i64,
) -> Result<JsonValue, String> {
	api.partner_chains_node()
		.reserve_deposit(amount_to_deposit, &payment_key(config))
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn test_deposit_funds() -> Result<(), String> {
		let config = ApiConfig::load();
		let api = &SubstrateApi::new(&config);
		let payment_key = payment_key(&config);
		api.partner_chains_node().reserve_init(&payment_key)?;
		create_reserve(&config, api)?;
		let amount_to_deposit = amount_to_deposit(api, &config)?;
		let initial_balance = reserve_initial_balance(api, &config)?;
		let native_token_balance = native_token_balance(api, &config)?;

		// def test_deposit_funds():
		let response = deposit_funds(api, &config, amount_to_deposit);
		assert!(response.is_ok());

		// def test_reserve_balance_after_deposit():
		let reserve_asset_id = &reserve_asset_id(&config, api)?;
		let reserve_balance = api.get_mc_balance(
			addresses(api)["ReserveValidator"].as_str().expect("str"),
			reserve_asset_id,
		)?;
		assert!(initial_balance + amount_to_deposit == reserve_balance);

		// def test_native_token_balance_after_deposit():
		let native_token = api.get_mc_balance(&governance_address(&config), reserve_asset_id)?;
		assert!(native_token_balance - amount_to_deposit == native_token);
		clean_up_reserve(&config, api)?;
		Ok(())
	}
}
