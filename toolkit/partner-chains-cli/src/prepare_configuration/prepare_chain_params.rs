use std::str::FromStr;

use crate::config::config_fields::{self, GENESIS_UTXO};
use crate::config::{CardanoNetwork, ConfigFieldDefinition, ServiceConfig};
use crate::io::IOContext;
use crate::ogmios::{OgmiosRequest, OgmiosResponse};
use crate::register::register1::ValidUtxo;
use crate::{cardano_key, pc_contracts_cli_resources};
use anyhow::anyhow;
use ogmios_client::types::OgmiosUtxo;
use serde::de::DeserializeOwned;
use sidechain_domain::{McTxHash, UtxoId};

use super::prepare_cardano_params::get_shelley_config;

pub fn prepare_chain_params<C: IOContext>(context: &C) -> anyhow::Result<(UtxoId, ServiceConfig)> {
	context.eprint(INTRO);
	context.print("This wizard will query your UTXOs using address derived from the payment verification key and Ogmios service");
	let ogmios_configuration = pc_contracts_cli_resources::prompt_ogmios_configuration(context)?;
	let shelley_config = get_shelley_config(&ogmios_configuration.to_string(), context)?;
	let address = derive_address(context, shelley_config.network)?;
	let utxo_query_result = query_utxos(context, &ogmios_configuration, &address)?;

	let valid_utxos: Vec<ValidUtxo> = filter_utxos(utxo_query_result);

	if valid_utxos.is_empty() {
		context.eprint("⚠️ No UTXOs found for the given address");
		context.eprint("There has to be at least one UTXO in the governance authority wallet.");
		return Err(anyhow::anyhow!("No UTXOs found"));
	};
	let utxo_display_options: Vec<String> =
		valid_utxos.iter().map(|utxo| utxo.to_display_string()).collect();
	let selected_utxo_display_string = context
		.prompt_multi_option("Select an UTXO to use as the genesis UTXO", utxo_display_options);

	let selected_utxo = valid_utxos
		.iter()
		.find(|utxo| utxo.to_display_string() == selected_utxo_display_string)
		.map(|utxo| utxo.utxo_id.to_string())
		.ok_or_else(|| anyhow!("⚠️ Failed to find selected UTXO"))?;

	let genesis_utxo: UtxoId = UtxoId::from_str(&selected_utxo).map_err(|e| {
		context.eprint(&format!("⚠️ Failed to parse selected UTXO: {e}"));
		anyhow!(e)
	})?;

	context.print(
		"Please do not spend this UTXO, it needs to be consumed by the governance initialization.",
	);
	context.print("");

	save_if_missing(GENESIS_UTXO, genesis_utxo, context);
	Ok((genesis_utxo, ogmios_configuration))
}

fn save_if_missing<T, C: IOContext>(field: ConfigFieldDefinition<'_, T>, new_value: T, context: &C)
where
	T: DeserializeOwned + std::fmt::Display + serde::Serialize,
{
	if field.load_from_file_and_print(context).is_none() {
		field.save_to_file(&new_value, context);
	}
}

fn query_utxos<C: IOContext>(
	context: &C,
	ogmios_config: &ServiceConfig,
	address: &str,
) -> Result<Vec<OgmiosUtxo>, anyhow::Error> {
	let ogmios_addr = ogmios_config.to_string();
	context.print(&format!("⚙️ Querying UTXOs of {address} from Ogmios at {ogmios_addr}..."));
	let response = context
		.ogmios_rpc(&ogmios_addr, OgmiosRequest::QueryUtxo { address: address.into() })
		.map_err(|e| anyhow!(e))?;
	match response {
		OgmiosResponse::QueryUtxo(utxos) => Ok(utxos),
		other => Err(anyhow::anyhow!(format!(
			"Unexpected response from Ogmios when querying for utxos: {other:?}"
		))),
	}
}

fn derive_address<C: IOContext>(
	context: &C,
	cardano_network: CardanoNetwork,
) -> Result<String, anyhow::Error> {
	let cardano_payment_verification_key_file =
		config_fields::CARDANO_PAYMENT_VERIFICATION_KEY_FILE
			.prompt_with_default_from_file_and_save(context);
	let key_bytes: [u8; 32] =
		cardano_key::get_key_bytes_from_file(&cardano_payment_verification_key_file, context)?;
	let address =
		partner_chains_cardano_offchain::csl::payment_address(&key_bytes, cardano_network.into());
	address.to_bech32(None).map_err(|e| anyhow!(e.to_string()))
}

// Take only the UTXOs without multi-asset tokens
fn filter_utxos(utxos: Vec<OgmiosUtxo>) -> Vec<ValidUtxo> {
	let mut utxos: Vec<ValidUtxo> = utxos
		.into_iter()
		.filter_map(|utxo| {
			if utxo.value.native_tokens.is_empty() {
				Some(ValidUtxo {
					utxo_id: UtxoId {
						tx_hash: McTxHash(utxo.transaction.id),
						index: sidechain_domain::UtxoIndex(utxo.index),
					},
					lovelace: utxo.value.lovelace,
				})
			} else {
				None
			}
		})
		.collect();

	utxos.sort_by_key(|utxo| std::cmp::Reverse(utxo.lovelace));
	utxos
}
const INTRO: &str = "Now, let's set up the genesis utxo. It identifies a partner chain.";

#[cfg(test)]
mod tests {
	use crate::config::config_fields::{CARDANO_PAYMENT_VERIFICATION_KEY_FILE, GENESIS_UTXO};
	use crate::config::RESOURCES_CONFIG_FILE_PATH;
	use crate::pc_contracts_cli_resources::default_ogmios_service_config;
	use crate::pc_contracts_cli_resources::tests::prompt_ogmios_configuration_io;
	use crate::prepare_configuration::prepare_chain_params::prepare_chain_params;
	use crate::prepare_configuration::tests::prompt_and_save_to_existing_file;
	use crate::tests::{MockIO, MockIOContext};

	fn test_vkey_file_json() -> serde_json::Value {
		serde_json::json!({
			"type": "PaymentVerificationKeyShelley_ed25519",
			"description": "Payment Verification Key",
			"cborHex": "5820516c971f57d5db063161b3240dfa95cdd8030242dd8756c5a003978b4113788c"
		})
	}

	mod scenarios {
		use crate::prepare_configuration::prepare_chain_params::INTRO;
		use crate::tests::MockIO;

		pub fn show_intro() -> MockIO {
			MockIO::Group(vec![MockIO::eprint(INTRO)])
		}
	}

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_file(RESOURCES_CONFIG_FILE_PATH, "{}")
			.with_json_file("payment.vkey", test_vkey_file_json())
			.with_expected_io(vec![
				scenarios::show_intro(),
				MockIO::print("This wizard will query your UTXOs using address derived from the payment verification key and Ogmios service"),
				prompt_ogmios_configuration_io(
					&default_ogmios_service_config(),
					&default_ogmios_service_config(),
				),
				prompt_and_save_to_existing_file(
					CARDANO_PAYMENT_VERIFICATION_KEY_FILE,
					"payment.vkey",
				),
				MockIO::file_write_json_contains(
					GENESIS_UTXO.config_file,
					&GENESIS_UTXO.json_pointer(),
					"0000000000000000000000000000000000000000000000000000000000000000#0",
				),
			]);

		let result = prepare_chain_params(&mock_context);

		result.expect("should succeed");
	}
}
