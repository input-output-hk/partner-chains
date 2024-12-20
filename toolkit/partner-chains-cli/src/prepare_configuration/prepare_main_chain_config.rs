use crate::config::config_fields::{
	COMMITTEE_CANDIDATES_ADDRESS, D_PARAMETER_POLICY_ID, ILLIQUID_SUPPLY_ADDRESS,
	INITIAL_PERMISSIONED_CANDIDATES, NATIVE_TOKEN_ASSET_NAME, NATIVE_TOKEN_POLICY,
	PERMISSIONED_CANDIDATES_POLICY_ID,
};
use crate::config::ServiceConfig;
use crate::io::IOContext;
use crate::prepare_configuration::prepare_cardano_params::prepare_cardano_params;
use partner_chains_cardano_offchain::scripts_data::GetScriptsData;
use sidechain_domain::{PolicyId, UtxoId};

pub fn prepare_main_chain_config<C: IOContext>(
	context: &C,
	ogmios_config: &ServiceConfig,
	genesis_utxo: UtxoId,
) -> anyhow::Result<()> {
	let cardano_parameteres = prepare_cardano_params(ogmios_config, context)?;
	cardano_parameteres.save(context);
	set_up_cardano_addresses(context, genesis_utxo, ogmios_config)?;

	if INITIAL_PERMISSIONED_CANDIDATES.load_from_file(context).is_none() {
		INITIAL_PERMISSIONED_CANDIDATES.save_to_file(&vec![], context)
	}
	prepare_native_token(context)?;
	context.eprint(OUTRO);
	Ok(())
}

fn set_up_cardano_addresses<C: IOContext>(
	context: &C,
	genesis_utxo: UtxoId,
	ogmios_config: &ServiceConfig,
) -> anyhow::Result<()> {
	let offchain_impl = context.offchain_impl(ogmios_config)?;
	let runtime = tokio::runtime::Runtime::new().map_err(|e| anyhow::anyhow!(e))?;
	let scripts_data = runtime
		.block_on(offchain_impl.get_scripts_data(genesis_utxo))
		.map_err(|e| anyhow::anyhow!("Offchain call failed: {e:?}!"))?;

	let committee_candidate_validator_addr = scripts_data.addresses.committee_candidate_validator;
	let d_parameter_policy_id = hex::encode(scripts_data.policy_ids.d_parameter.0);
	let permissioned_candidates_policy_id =
		hex::encode(scripts_data.policy_ids.permissioned_candidates.0);
	let illiquid_supply_addr = scripts_data.addresses.illiquid_circulation_supply_validator;
	COMMITTEE_CANDIDATES_ADDRESS.save_to_file(&committee_candidate_validator_addr, context);
	D_PARAMETER_POLICY_ID.save_to_file(&d_parameter_policy_id, context);
	PERMISSIONED_CANDIDATES_POLICY_ID.save_to_file(&permissioned_candidates_policy_id, context);
	ILLIQUID_SUPPLY_ADDRESS.save_to_file(&illiquid_supply_addr, context);
	context.print(&format!(
		"Cardano addresses have been set up:
- Committee Candidates Address: {committee_candidate_validator_addr}
- D Parameter Policy ID: {d_parameter_policy_id}
- Permissioned Candidates Policy ID: {permissioned_candidates_policy_id}
- Illiquid Supply Address: {illiquid_supply_addr}"
	));
	Ok(())
}

fn prepare_native_token<C: IOContext>(context: &C) -> anyhow::Result<()> {
	context.print(
		"Partner Chains can store their initial token supply on Cardano as Cardano native tokens.",
	);
	context.print("Creation of the native token is not supported by this wizard and must be performed manually before this step.");
	if context.prompt_yes_no("Do you want to configure a native token for you Partner Chain?", true)
	{
		NATIVE_TOKEN_POLICY.prompt_with_default_from_file_and_save(context);
		NATIVE_TOKEN_ASSET_NAME.prompt_with_default_from_file_and_save(context);
	} else {
		NATIVE_TOKEN_POLICY.save_to_file(&PolicyId::default().to_hex_string(), context);
		NATIVE_TOKEN_ASSET_NAME.save_to_file(&"0x".into(), context);
	}

	Ok(())
}

const OUTRO: &str = r#"Chain configuration (partner-chains-cli-chain-config.json) is now ready for distribution to network participants.

If you intend to run a chain with permissioned candidates, you must manually set their keys in the partner-chains-cli-chain-config.json file before proceeding. Here's an example of how to add permissioned candidates:

{
  ...
  "initial_permissioned_candidates": [
    {
      "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde49a5684e7a56da27d",
      "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200f498922423d4334014fa6b0ee",
      "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e0477968906ac916b04cb365ec3153755684d9a1"
    },
    {
      "aura_pub_key": "0x8eaf04151687736326c9fea17e25fc5287613698c912909cb226aa4794f26a48",
      "grandpa_pub_key": "0xd17c2d7823ebf260fd138f2d7e27d114cb145d968b5ff5006125f2414fadae69",
      "sidechain_pub_key": "0x0390084fdbf27d2b79d26a4f13f0cdd982cb755a661969143c37cbc49ef5b91f27"
    }
  ]
}

After setting up the permissioned candidates, execute the 'create-chain-spec' command to generate the final chain specification."#;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::config_fields::{GENESIS_UTXO, OGMIOS_PROTOCOL};
	use crate::config::NetworkProtocol;
	use crate::ogmios::test_values::{preprod_eras_summaries, preprod_shelley_config};
	use crate::ogmios::{OgmiosRequest, OgmiosResponse};
	use crate::prepare_configuration::prepare_cardano_params::tests::PREPROD_CARDANO_PARAMS;
	use crate::prepare_configuration::tests::save_to_existing_file;
	use crate::tests::{MockIO, MockIOContext, OffchainMock, OffchainMocks};
	use partner_chains_cardano_offchain::scripts_data::{Addresses, PolicyIds, ScriptsData};
	use serde_json::json;
	use serde_json::Value;
	use sidechain_domain::UtxoId;
	use std::str::FromStr;

	const TEST_GENESIS_UTXO: &str =
		"0000000000000000000000000000000000000000000000000000000000000000#0";
	const TEST_D_PARAMETER_POLICY_ID: &str =
		"623cc9d41321674962b8599bf2baf0f34b8df8ad9d549f7ba3b1fdbb";
	const TEST_COMMITTEE_CANDIDATES_ADDRESS: &str =
		"addr_test1wz5fe8fmxx4v83gzfsdlnhgxm8x7zpldegrqh2wakl3wteqe834r4";
	const TEST_PERMISSIONED_CANDIDATES_POLICY_ID: &str =
		"13db1ba564b3b264f45974fece44b2beb0a2326b10e65a0f7f300dfb";
	const TEST_ILLIQUID_SUPPLY_ADDRESS: &str =
		"addr_test1wqn2pkvvmesmxtfa4tz7w8gh8vumr52lpkrhcs4dkg30uqq77h5z4";

	fn ogmios_config() -> ServiceConfig {
		ServiceConfig {
			hostname: "localhost".to_string(),
			port: 1337,
			protocol: NetworkProtocol::Http,
		}
	}

	pub mod scenarios {
		use super::*;
		use crate::config::config_fields::*;

		pub fn save_cardano_params() -> MockIO {
			MockIO::Group(vec![
				save_to_existing_file(
					CARDANO_SECURITY_PARAMETER,
					&PREPROD_CARDANO_PARAMS.security_parameter.to_string(),
				),
				save_to_existing_file(
					CARDANO_ACTIVE_SLOTS_COEFF,
					&PREPROD_CARDANO_PARAMS.active_slots_coeff.to_string(),
				),
				save_to_existing_file(
					CARDANO_FIRST_EPOCH_NUMBER,
					&PREPROD_CARDANO_PARAMS.first_epoch_number.to_string(),
				),
				save_to_existing_file(
					CARDANO_FIRST_SLOT_NUMBER,
					&PREPROD_CARDANO_PARAMS.first_slot_number.to_string(),
				),
				save_to_existing_file(
					CARDANO_EPOCH_DURATION_MILLIS,
					&PREPROD_CARDANO_PARAMS.epoch_duration_millis.to_string(),
				),
				save_to_existing_file(
					CARDANO_FIRST_EPOCH_TIMESTAMP_MILLIS,
					&PREPROD_CARDANO_PARAMS.first_epoch_timestamp_millis.to_string(),
				),
			])
		}

		pub fn prompt_and_save_native_asset_scripts() -> MockIO {
			MockIO::Group(vec![
						MockIO::print("Partner Chains can store their initial token supply on Cardano as Cardano native tokens."),
						MockIO::print("Creation of the native token is not supported by this wizard and must be performed manually before this step."),
						MockIO::prompt_yes_no(
							"Do you want to configure a native token for you Partner Chain?",
							true,
							true,
						),
						MockIO::file_read(NATIVE_TOKEN_POLICY.config_file),
						MockIO::prompt(
							NATIVE_TOKEN_POLICY.name,
							None,
							"ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
						),
						MockIO::file_read(NATIVE_TOKEN_POLICY.config_file),
						MockIO::file_write_json_contains(
							NATIVE_TOKEN_POLICY.config_file,
							&NATIVE_TOKEN_POLICY.json_pointer(),
							"ada83ddd029614381f00e28de0922ab0dec6983ea9dd29ae20eef9b4",
						),
						MockIO::file_read(NATIVE_TOKEN_ASSET_NAME.config_file),
						MockIO::prompt(NATIVE_TOKEN_ASSET_NAME.name, None, "5043546f6b656e44656d6f"),
						MockIO::file_read(NATIVE_TOKEN_ASSET_NAME.config_file),
						MockIO::file_write_json_contains(
							NATIVE_TOKEN_ASSET_NAME.config_file,
							&NATIVE_TOKEN_ASSET_NAME.json_pointer(),
							"5043546f6b656e44656d6f",
						),
					])
		}
	}

	fn payment_key_content() -> serde_json::Value {
		json!({
			"type": "PaymentSigningKeyShelley_ed25519",
			"description": "Payment Signing Key",
			"cborHex": "5820d0a6c5c921266d15dc8d1ce1e51a01e929a686ed3ec1a9be1145727c224bf386"
		})
	}

	#[test]
	fn happy_path() {
		let mock_context = MockIOContext::new()
			.with_json_file(GENESIS_UTXO.config_file, serde_json::json!({}))
			.with_json_file(OGMIOS_PROTOCOL.config_file, serde_json::json!({}))
			.with_json_file("payment.skey", payment_key_content())
			.with_offchain_mocks(preprod_offchain_mocks())
			.with_expected_io(vec![
				MockIO::ogmios_request(
					"http://localhost:1337",
					OgmiosRequest::QueryLedgerStateEraSummaries,
					Ok(OgmiosResponse::QueryLedgerStateEraSummaries(preprod_eras_summaries())),
				),
				MockIO::ogmios_request(
					"http://localhost:1337",
					OgmiosRequest::QueryNetworkShelleyGenesis,
					Ok(OgmiosResponse::QueryNetworkShelleyGenesis(preprod_shelley_config())),
				),
				scenarios::save_cardano_params(),
				save_to_existing_file(
					COMMITTEE_CANDIDATES_ADDRESS,
					TEST_COMMITTEE_CANDIDATES_ADDRESS,
				),
				save_to_existing_file(D_PARAMETER_POLICY_ID, TEST_D_PARAMETER_POLICY_ID),
				save_to_existing_file(
					PERMISSIONED_CANDIDATES_POLICY_ID,
					TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
				),
				save_to_existing_file(ILLIQUID_SUPPLY_ADDRESS, TEST_ILLIQUID_SUPPLY_ADDRESS),
				print_addresses_io(),
				MockIO::file_read(INITIAL_PERMISSIONED_CANDIDATES.config_file),
				MockIO::file_read(INITIAL_PERMISSIONED_CANDIDATES.config_file),
				MockIO::file_write_json(
					INITIAL_PERMISSIONED_CANDIDATES.config_file,
					test_chain_config(),
				),
				scenarios::prompt_and_save_native_asset_scripts(),
				MockIO::eprint(OUTRO),
			]);
		prepare_main_chain_config(
			&mock_context,
			&ogmios_config(),
			UtxoId::from_str(TEST_GENESIS_UTXO).unwrap(),
		)
		.expect("should succeed");
	}

	#[test]
	fn happy_path_with_initial_permissioned_candidates() {
		let mock_context = MockIOContext::new()
			.with_json_file(
				INITIAL_PERMISSIONED_CANDIDATES.config_file,
				serde_json::json!({
					"initial_permissioned_candidates": [
						{
						  "aura_pub_key": "0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
						  "grandpa_pub_key": "0x88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee",
						  "sidechain_pub_key": "0x020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1"
						}
					]
				}),
			)
			.with_json_file("payment.skey", payment_key_content())
			.with_json_file(OGMIOS_PROTOCOL.config_file, serde_json::json!({}))
			.with_offchain_mocks(preprod_offchain_mocks())
			.with_expected_io(vec![
				MockIO::ogmios_request(
					"http://localhost:1337",
					OgmiosRequest::QueryLedgerStateEraSummaries,
					Ok(OgmiosResponse::QueryLedgerStateEraSummaries(preprod_eras_summaries())),
				),
				MockIO::ogmios_request(
					"http://localhost:1337",
					OgmiosRequest::QueryNetworkShelleyGenesis,
					Ok(OgmiosResponse::QueryNetworkShelleyGenesis(preprod_shelley_config())),
				),
				scenarios::save_cardano_params(),
				save_to_existing_file(
					COMMITTEE_CANDIDATES_ADDRESS,
					TEST_COMMITTEE_CANDIDATES_ADDRESS,
				),
				save_to_existing_file(D_PARAMETER_POLICY_ID, TEST_D_PARAMETER_POLICY_ID),
				save_to_existing_file(
					PERMISSIONED_CANDIDATES_POLICY_ID,
					TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
				),
				save_to_existing_file(ILLIQUID_SUPPLY_ADDRESS, TEST_ILLIQUID_SUPPLY_ADDRESS),
				print_addresses_io(),
				MockIO::file_read(INITIAL_PERMISSIONED_CANDIDATES.config_file),
				scenarios::prompt_and_save_native_asset_scripts(),
				MockIO::eprint(OUTRO),
			]);
		prepare_main_chain_config(
			&mock_context,
			&ogmios_config(),
			UtxoId::from_str(TEST_GENESIS_UTXO).unwrap(),
		)
		.expect("should succeed");
	}

	fn print_addresses_io() -> MockIO {
		MockIO::print(&format!(
			"Cardano addresses have been set up:
- Committee Candidates Address: {TEST_COMMITTEE_CANDIDATES_ADDRESS}
- D Parameter Policy ID: {TEST_D_PARAMETER_POLICY_ID}
- Permissioned Candidates Policy ID: {TEST_PERMISSIONED_CANDIDATES_POLICY_ID}
- Illiquid Supply Address: {TEST_ILLIQUID_SUPPLY_ADDRESS}",
		))
	}

	fn preprod_offchain_mocks() -> OffchainMocks {
		let mock = OffchainMock::new().with_scripts_data(
			UtxoId::from_str(TEST_GENESIS_UTXO).unwrap(),
			Ok(ScriptsData {
				addresses: Addresses {
					committee_candidate_validator: TEST_COMMITTEE_CANDIDATES_ADDRESS.to_string(),
					illiquid_circulation_supply_validator: TEST_ILLIQUID_SUPPLY_ADDRESS.to_string(),
					..Default::default()
				},
				policy_ids: PolicyIds {
					permissioned_candidates: PolicyId::from_hex_unsafe(
						TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
					),
					d_parameter: PolicyId::from_hex_unsafe(TEST_D_PARAMETER_POLICY_ID),
					..Default::default()
				},
			}),
		);
		OffchainMocks::new_with_mock("http://localhost:1337", mock)
	}

	fn test_chain_config() -> Value {
		serde_json::json!({
			"cardano": {
				"security_parameter": PREPROD_CARDANO_PARAMS.security_parameter,
				"active_slots_coeff": PREPROD_CARDANO_PARAMS.active_slots_coeff,
				"first_epoch_number": PREPROD_CARDANO_PARAMS.first_epoch_number,
				"first_slot_number": PREPROD_CARDANO_PARAMS.first_slot_number,
				"epoch_duration_millis": PREPROD_CARDANO_PARAMS.epoch_duration_millis,
				"first_epoch_timestamp_millis": PREPROD_CARDANO_PARAMS.first_epoch_timestamp_millis
			},
			"cardano_addresses": {
				"committee_candidates_address": TEST_COMMITTEE_CANDIDATES_ADDRESS,
				"d_parameter_policy_id": TEST_D_PARAMETER_POLICY_ID,
				"permissioned_candidates_policy_id": TEST_PERMISSIONED_CANDIDATES_POLICY_ID,
				"native_token": {
					"illiquid_supply_address": TEST_ILLIQUID_SUPPLY_ADDRESS,
				}
			},
			"initial_permissioned_candidates": []
		})
	}
}
