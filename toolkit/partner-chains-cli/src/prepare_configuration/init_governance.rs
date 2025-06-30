use super::InitGovernance;
use crate::{
	IOContext,
	config::{
		GovernanceAuthoritiesKeyHashes, ServiceConfig,
		config_fields::{INITIAL_GOVERNANCE_AUTHORITIES, INITIAL_GOVERNANCE_THRESHOLD},
	},
};
use anyhow::anyhow;
use partner_chains_cardano_offchain::{
	await_tx::FixedDelayRetries, cardano_keys::CardanoPaymentSigningKey,
	governance::MultiSigParameters,
};
use sidechain_domain::{MainchainKeyHash, McTxHash, UtxoId};

pub(crate) fn run_init_governance<C: IOContext>(
	await_tx: FixedDelayRetries,
	genesis_utxo: UtxoId,
	payment_key: &CardanoPaymentSigningKey,
	ogmios_config: &ServiceConfig,
	context: &C,
) -> anyhow::Result<Option<McTxHash>> {
	let multisig_parameters = prompt_initial_governance(payment_key, context)?;
	let should_continue = context.prompt_yes_no(
		&format!(
			"Governance will be initialized with:\n{}\nDo you want to continue?",
			multisig_parameters
		),
		false,
	);
	if should_continue {
		let offchain = context.offchain_impl(ogmios_config)?;
		let runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
		let tx_id = runtime
			.block_on(offchain.init_governance(
				await_tx,
				&multisig_parameters,
				&payment_key,
				genesis_utxo,
			))
			.map_err(|e| anyhow::anyhow!("Governance initialization failed: {e:?}!"))?;
		context.eprint(&format!("Governance initialized successfully for UTXO: {}", genesis_utxo));
		Ok(Some(tx_id))
	} else {
		Ok(None)
	}
}

fn prompt_initial_governance<C: IOContext>(
	payment_key: &CardanoPaymentSigningKey,
	context: &C,
) -> Result<MultiSigParameters, anyhow::Error> {
	context.eprint("Please provide the initial chain governance key hashes:");
	INITIAL_GOVERNANCE_AUTHORITIES.save_if_empty(
		GovernanceAuthoritiesKeyHashes(vec![payment_key.to_pub_key_hash()]),
		context,
	);
	let authorities = INITIAL_GOVERNANCE_AUTHORITIES
		.prompt_with_default_from_file_parse_and_save(context)
		.map_err(|e| anyhow!("Failed to parse governance authorities: {}", e))?;

	INITIAL_GOVERNANCE_THRESHOLD.save_if_empty(1, context);
	let threshold = INITIAL_GOVERNANCE_THRESHOLD
		.prompt_with_default_from_file_parse_and_save(context)
		.map_err(|e| anyhow!("Failed do parse threshold: {}", e))?;

	MultiSigParameters::new(authorities.0.as_ref(), threshold).map_err(
		|err| anyhow!(
			"Initial Governance data is invalid: '{}'. Please run the wizard again and provide correct value or edit values in '{}' and the run the wizard again. Example: '{}'",
			err,
			INITIAL_GOVERNANCE_AUTHORITIES.config_file,
			&example_governance_auth()
		)
	)
}

fn example_governance_auth() -> serde_json::Value {
	serde_json::json!({
		"initial_governance": {
			"authorities" : [
				MainchainKeyHash([0u8;28]),
				MainchainKeyHash([1u8;28]),
				MainchainKeyHash([2u8;28])
			],
			"threshold": 2
		}
	})
}

#[cfg(test)]
mod tests {
	use super::run_init_governance;
	use crate::{
		config::{
			CHAIN_CONFIG_FILE_PATH, NetworkProtocol, ServiceConfig,
			config_fields::{GENESIS_UTXO, OGMIOS_PROTOCOL},
		},
		tests::{MockIO, MockIOContext, OffchainMock, OffchainMocks},
		verify_json,
	};
	use hex_literal::hex;
	use partner_chains_cardano_offchain::{
		await_tx::FixedDelayRetries, cardano_keys::CardanoPaymentSigningKey,
		governance::MultiSigParameters,
	};
	use serde_json::{Value, json};
	use sidechain_domain::{MainchainKeyHash, McTxHash, UtxoId};

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_json_file(GENESIS_UTXO.config_file, serde_json::json!({}))
			.with_json_file(OGMIOS_PROTOCOL.config_file, serde_json::json!({}))
			.with_offchain_mocks(preprod_offchain_mocks())
			.with_expected_io(vec![
				MockIO::eprint("Please provide the initial chain governance key hashes:"),
				MockIO::prompt(
					"Enter the space separated keys hashes of the initial Multisig Governance Authorities",
					Some(test_private_key_hash()),
					"00000000000000000000000000000000000000000000000000000000  \n\t0x01010101010101010101010101010101010101010101010101010101",
				),
				MockIO::prompt(
					"Enter the Initial Multisig Governance Threshold",
					Some("1"),
					"2",
				),
				MockIO::prompt_yes_no(
"Governance will be initialized with:\
\nGovernance authorities:\
\n\t0x00000000000000000000000000000000000000000000000000000000\
\n\t0x01010101010101010101010101010101010101010101010101010101\
\nThreshold: 2\
\nDo you want to continue?",false, true),
				MockIO::eprint("Governance initialized successfully for UTXO: 0000000000000000000000000000000000000000000000000000000000000000#0")
			]);
		run_init_governance(
			FixedDelayRetries::five_minutes(),
			TEST_GENESIS_UTXO,
			&test_private_key(),
			&ogmios_config(),
			&mock_context,
		)
		.expect("should succeed");
		verify_json!(mock_context, CHAIN_CONFIG_FILE_PATH, expected_chain_config_content());
	}

	const TEST_GENESIS_UTXO: UtxoId = UtxoId::new([0u8; 32], 0);

	fn ogmios_config() -> ServiceConfig {
		ServiceConfig {
			hostname: "localhost".to_string(),
			port: 1337,
			protocol: NetworkProtocol::Http,
			timeout_seconds: 180,
		}
	}

	fn test_private_key() -> CardanoPaymentSigningKey {
		CardanoPaymentSigningKey::from_normal_bytes(hex!(
			"d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"
		))
		.unwrap()
	}

	fn test_private_key_hash() -> &'static str {
		"0xe8c300330fe315531ca89d4a2e7d0c80211bc70b473b1ed4979dff2b"
	}

	fn preprod_offchain_mocks() -> OffchainMocks {
		let mock = OffchainMock::new().with_init_governance(
			TEST_GENESIS_UTXO,
			MultiSigParameters::new(
				&[
					MainchainKeyHash(hex!(
						"00000000000000000000000000000000000000000000000000000000"
					)),
					MainchainKeyHash(hex!(
						"01010101010101010101010101010101010101010101010101010101"
					)),
				],
				2u8,
			)
			.unwrap(),
			hex!("d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386").to_vec(),
			Ok(McTxHash(hex!("2222222200000000000000000000000000000000000000000000000000000000"))),
		);
		OffchainMocks::new_with_mock("http://localhost:1337", mock)
	}

	fn expected_chain_config_content() -> Value {
		json!({
			"initial_governance":  {
				"authorities":  [
					"0x00000000000000000000000000000000000000000000000000000000",
					"0x01010101010101010101010101010101010101010101010101010101"
				],
				"threshold": 2
			}
		})
	}
}
