use crate::chain_spec::pc_create_chain_spec;
use partner_chains_cli::{CreateChainSpecConfig, ParsedPermissionedCandidatesKeys};
use partner_chains_demo_runtime::opaque::SessionKeys;
use sidechain_domain::{AssetName, MainchainAddress, PolicyId, UtxoId};
use sp_core::{ecdsa, ed25519, sr25519};
use std::str::FromStr;

#[test]
fn pc_create_chain_spec_test() {
	let config = CreateChainSpecConfig {
		genesis_utxo: UtxoId::new([1u8; 32], 7),
		initial_permissioned_candidates_raw: vec![],
		initial_permissioned_candidates_parsed: vec![ParsedPermissionedCandidatesKeys {
			sidechain: ecdsa::Public::from_raw([11u8; 33]),
			keys: SessionKeys {
				aura: sr25519::Public::from([12u8; 32]).into(),
				grandpa: ed25519::Public::from([13u8; 32]).into(),
			},
		}],
		committee_candidate_address: MainchainAddress::from_str("addr_cca").unwrap(),
		d_parameter_policy_id: PolicyId([2u8; 28]),
		permissioned_candidates_policy_id: PolicyId([3u8; 28]),
		native_token_policy: PolicyId([4u8; 28]),
		native_token_asset_name: AssetName(vec![4, 4, 4].try_into().unwrap()),
		illiquid_supply_address: MainchainAddress::from_str("addr_nativetokenisc").unwrap(),
		governed_map_validator_address: Some(MainchainAddress::from_str("addr_govmap").unwrap()),
		governed_map_asset_policy_id: Some(PolicyId([5u8; 28])),
	};

	let json = pc_create_chain_spec(&config);
	let config = json.pointer("/genesis/runtimeGenesis/config").unwrap().clone();
	let config_obj = config.as_object().unwrap().clone();

	assert_eq!(
		config_obj.get("governedMap").unwrap(),
		&serde_json::json!({
		  "mainChainScripts": {
			"asset_policy_id": "0x05050505050505050505050505050505050505050505050505050505",
			"validator_address": "addr_govmap"
		  },
		  "marker": null
		})
	);

	assert_eq!(
		config_obj.get("nativeTokenManagement").unwrap(),
		&serde_json::json!({
		  "mainChainScripts": {
			"native_token_policy_id": "0x04040404040404040404040404040404040404040404040404040404",
			"native_token_asset_name": "0x040404",
			"illiquid_supply_validator_address": "addr_nativetokenisc"
		  },
		  "marker": null
		})
	);

	assert_eq!(
		config_obj.get("sidechain").unwrap(),
		&serde_json::json!({
			"genesisUtxo": "0101010101010101010101010101010101010101010101010101010101010101#7",
			"slotsPerEpoch": 60
		})
	);

	assert_eq!(
		config_obj.get("sessionCommitteeManagement").unwrap(),
		&serde_json::json!({
			"initialAuthorities": [
				{
					"Permissioned": {
						"id": "KWwG4siyRHiZtvV2nQSu6AHqjD68frQZwfZhT8zt7LWXYD64F",
						"keys": {
							"aura": "5CLW1ZaVdZdj6bf7nmvJfba6GbvxueXzV6Dw5fnPaKTiSARx",
							"grandpa": "5CMpMdu3LbHuj2TqX4RAUzXCHCqmNj8Fce43wAbcqSFZuNfp"
						}
					}
				}
			],
			"mainChainScripts": {
				"committee_candidate_address": "addr_cca",
				"d_parameter_policy_id": "0x02020202020202020202020202020202020202020202020202020202",
				"permissioned_candidates_policy_id": "0x03030303030303030303030303030303030303030303030303030303"
			}
		})
	);

	assert_eq!(
		config_obj.get("session").unwrap(),
		&serde_json::json!({
			"initialValidators": [
				[
					"5CUUBrDiVEKVa655Bsm8sYc5An5Jqi52PetteUpMY2JFbuRF",
					{
						"aura": "5CLW1ZaVdZdj6bf7nmvJfba6GbvxueXzV6Dw5fnPaKTiSARx",
						"grandpa": "5CMpMdu3LbHuj2TqX4RAUzXCHCqmNj8Fce43wAbcqSFZuNfp"
					}
				]
			]
		})
	);
}
