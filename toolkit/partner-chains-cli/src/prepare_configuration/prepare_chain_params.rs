use crate::config::config_fields::{self, GENESIS_UTXO};
use crate::config::ConfigFieldDefinition;
use crate::io::IOContext;
use anyhow::{anyhow, Context};
use serde::de::DeserializeOwned;
use sidechain_domain::UtxoId;

pub fn prepare_chain_params<C: IOContext>(context: &C) -> anyhow::Result<UtxoId> {
	context.eprint(INTRO);
	let _cardano_payment_verification_key_file =
		config_fields::CARDANO_PAYMENT_VERIFICATION_KEY_FILE
			.prompt_with_default_from_file_and_save(context);
	// TODO: replace this with prompt for the user's UTXOs
	// See register1 for an example
	let genesis_utxo = GENESIS_UTXO
		.default
		.ok_or(anyhow!("Genesis utxo should have a default value"))?
		.parse()
		.map_err(anyhow::Error::msg)
		.context("Genesis utxo should have a valid default value")?;

	save_if_missing(GENESIS_UTXO, genesis_utxo, context);
	Ok(genesis_utxo)
}

fn save_if_missing<T, C: IOContext>(field: ConfigFieldDefinition<'_, T>, new_value: T, context: &C)
where
	T: DeserializeOwned + std::fmt::Display + serde::Serialize,
{
	if field.load_from_file_and_print(context).is_none() {
		field.save_to_file(&new_value, context);
	}
}

const INTRO: &str = "Now, let's set up the genesis utxo. It identifies a partner chain.";

#[cfg(test)]
mod tests {
	use crate::config::config_fields::{CARDANO_PAYMENT_VERIFICATION_KEY_FILE, GENESIS_UTXO};
	use crate::config::RESOURCES_CONFIG_FILE_PATH;
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
