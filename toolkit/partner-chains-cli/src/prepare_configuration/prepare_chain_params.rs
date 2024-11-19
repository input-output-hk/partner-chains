use crate::config::config_fields::{self, GENESIS_UTXO};
use crate::config::{CardanoNetwork, ConfigFieldDefinition, ServiceConfig};
use crate::io::IOContext;
use crate::select_utxo::{filter_utxos, query_utxos, select_from_utxos, ValidUtxo};
use crate::{cardano_key, pc_contracts_cli_resources};
use anyhow::anyhow;
use serde::de::DeserializeOwned;
use sidechain_domain::UtxoId;

use super::prepare_cardano_params::get_shelley_config;

pub fn prepare_chain_params<C: IOContext>(context: &C) -> anyhow::Result<(UtxoId, ServiceConfig)> {
	context.eprint(INTRO);
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
	let genesis_utxo =
		select_from_utxos(context, "Select an UTXO to use as the genesis UTXO", valid_utxos)?;

	context.print(CAUTION);

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

const INTRO: &str = "Now, let's set up the genesis utxo. It identifies a partner chain. This wizard will query Ogmios for your UTXOs using address derived from the payment verification key. Please provide required data.";
const CAUTION: &str =
	"Please do not spend this UTXO, it needs to be consumed by the governance initialization.\n";

#[cfg(test)]
mod tests {
	use crate::config::config_fields::GENESIS_UTXO;
	use crate::config::RESOURCES_CONFIG_FILE_PATH;
	use crate::ogmios::{OgmiosRequest, OgmiosResponse};
	use crate::pc_contracts_cli_resources::default_ogmios_service_config;
	use crate::pc_contracts_cli_resources::tests::prompt_ogmios_configuration_io;
	use crate::prepare_configuration::prepare_cardano_params::tests::preview_shelley_config;
	use crate::prepare_configuration::prepare_chain_params::{
		prepare_chain_params, CAUTION, INTRO,
	};
	use crate::select_utxo::tests::{mock_5_valid_utxos_rows, mock_result_5_valid, query_utxos_io};
	use crate::tests::{MockIO, MockIOContext};

	fn test_vkey_file_json() -> serde_json::Value {
		serde_json::json!({
			"type": "PaymentVerificationKeyShelley_ed25519",
			"description": "Payment Verification Key",
			"cborHex": "5820516c971f57d5db063161b3240dfa95cdd8030242dd8756c5a003978b4113788c"
		})
	}

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_file(RESOURCES_CONFIG_FILE_PATH, "{}")
			.with_json_file("payment.vkey", test_vkey_file_json())
			.with_expected_io(vec![
				MockIO::eprint(INTRO),
				read_shelly_config_to_get_network(),
				prompt_payment_vkey_and_read_it_to_derive_address(),
				query_utxos_io(
					"addr_test1vpmd59ajuvm34d723r8q2qzyz9ylq0x9pygqn7vun8qgpkgs7y5hw",
					"http://localhost:1337",
					mock_result_5_valid(),
				),
				MockIO::prompt_multi_option(
					"Select an UTXO to use as the genesis UTXO",
				 	mock_5_valid_utxos_rows(),
					 "4704a903b01514645067d851382efd4a6ed5d2ff07cf30a538acc78fed7c4c02#93 (1100000 lovelace)"
				),
				MockIO::print(CAUTION),
				MockIO::file_write_json_contains(
					GENESIS_UTXO.config_file,
					&GENESIS_UTXO.json_pointer(),
					"4704a903b01514645067d851382efd4a6ed5d2ff07cf30a538acc78fed7c4c02#93",
				),
			]);

		let result = prepare_chain_params(&mock_context);

		result.expect("should succeed");
	}

	fn read_shelly_config_to_get_network() -> MockIO {
		MockIO::Group(vec![
			prompt_ogmios_configuration_io(
				&default_ogmios_service_config(),
				&default_ogmios_service_config(),
			),
			MockIO::ogmios_request(
				"http://localhost:1337",
				OgmiosRequest::QueryNetworkShelleyGenesis,
				Ok(OgmiosResponse::QueryNetworkShelleyGenesis(preview_shelley_config())),
			),
		])
	}

	fn prompt_payment_vkey_and_read_it_to_derive_address() -> MockIO {
		MockIO::Group(vec![
			MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
			MockIO::prompt(
				"path to the payment verification file",
				Some("payment.vkey"),
				"payment.vkey",
			),
			MockIO::file_read(RESOURCES_CONFIG_FILE_PATH),
			MockIO::file_write_json_contains(
				RESOURCES_CONFIG_FILE_PATH,
				"/cardano_payment_verification_key_file",
				"payment.vkey",
			),
			MockIO::file_read("payment.vkey"),
		])
	}
}
