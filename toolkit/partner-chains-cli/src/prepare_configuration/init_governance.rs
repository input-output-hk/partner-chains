use crate::{
	cardano_key,
	config::{config_fields, ServiceConfig},
	IOContext,
};
use ogmios_client::types::OgmiosTx;
use partner_chains_cardano_offchain::{
	cardano_keys::CardanoSigningKey, init_governance::InitGovernance,
};
use sidechain_domain::{MainchainKeyHash, UtxoId};

pub(crate) fn run_init_governance<C: IOContext>(
	genesis_utxo: UtxoId,
	ogmios_config: &ServiceConfig,
	context: &C,
) -> anyhow::Result<OgmiosTx> {
	let offchain = context.offchain_impl(ogmios_config)?;
	let (payment_key, governance_authority) = get_private_key_and_key_hash(context)?;
	let runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
	runtime
		.block_on(offchain.init_governance(governance_authority, &payment_key, genesis_utxo))
		.map_err(|e| anyhow::anyhow!("Governance initalization failed: {e:?}!"))
}

fn get_private_key_and_key_hash<C: IOContext>(
	context: &C,
) -> Result<(CardanoSigningKey, MainchainKeyHash), anyhow::Error> {
	let cardano_signing_key_file = config_fields::CARDANO_PAYMENT_SIGNING_KEY_FILE
		.prompt_with_default_from_file_and_save(context);
	let pkey =
		cardano_key::get_mc_payment_signing_key_from_file(&cardano_signing_key_file, context)?;
	let addr_hash = pkey.to_pub_key_hash();

	Ok((pkey, addr_hash))
}

#[cfg(test)]
mod tests {
	use super::run_init_governance;
	use crate::{
		config::{
			config_fields::{GENESIS_UTXO, OGMIOS_PROTOCOL},
			NetworkProtocol, ServiceConfig, RESOURCES_CONFIG_FILE_PATH,
		},
		tests::{MockIO, MockIOContext, OffchainMock, OffchainMocks},
		verify_json,
	};
	use hex_literal::hex;
	use ogmios_client::types::OgmiosTx;
	use serde_json::{json, Value};
	use sidechain_domain::{MainchainKeyHash, UtxoId};

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_json_file(GENESIS_UTXO.config_file, serde_json::json!({}))
			.with_json_file(OGMIOS_PROTOCOL.config_file, serde_json::json!({}))
			.with_json_file("payment.skey", payment_key_content())
			.with_offchain_mocks(preprod_offchain_mocks())
			.with_expected_io(vec![MockIO::prompt(
				"path to the payment signing key file",
				Some("payment.skey"),
				"payment.skey",
			)]);
		run_init_governance(TEST_GENESIS_UTXO, &ogmios_config(), &mock_context)
			.expect("should succeed");
		verify_json!(mock_context, RESOURCES_CONFIG_FILE_PATH, test_resources_config());
	}

	fn payment_key_content() -> serde_json::Value {
		json!({
			"type": "PaymentSigningKeyShelley_ed25519",
			"description": "Payment Signing Key",
			"cborHex": "5820d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"
		})
	}
	const TEST_GENESIS_UTXO: UtxoId = UtxoId::new([0u8; 32], 0);

	fn ogmios_config() -> ServiceConfig {
		ServiceConfig {
			hostname: "localhost".to_string(),
			port: 1337,
			protocol: NetworkProtocol::Http,
		}
	}

	fn test_resources_config() -> Value {
		serde_json::json!({
				"cardano_payment_signing_key_file": "payment.skey",
		})
	}

	fn preprod_offchain_mocks() -> OffchainMocks {
		let mock = OffchainMock::new().with_init_governance(
			TEST_GENESIS_UTXO,
			MainchainKeyHash(hex!("e8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b")),
			hex!("d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386").to_vec(),
			Ok(OgmiosTx {
				id: hex!("0000000000000000000000000000000000000000000000000000000000000000"),
			}),
		);
		OffchainMocks::new_with_mock("http://localhost:1337", mock)
	}
}
