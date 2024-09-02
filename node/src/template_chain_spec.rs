use crate::chain_spec::*;
use chain_params::SidechainParams;
use sc_service::ChainType;
use sidechain_domain::{MainchainAddressHash, UtxoId};
use sidechain_runtime::{
	AuraConfig, BalancesConfig, GrandpaConfig, NativeTokenManagementConfig, RuntimeGenesisConfig,
	SessionCommitteeManagementConfig, SessionConfig, SidechainConfig, SudoConfig, SystemConfig,
};

/// Produces template chain spec for Partner Chains.
/// This code should be run by `partner-chains-cli chain-spec`, to produce JSON chain spec file.
/// `initial_validators` fields should be updated by the `partner-chains-cli chain-spec`.
/// Add and modify other fields of `ChainSpec` accordingly to the needs of your chain.
pub fn chain_spec() -> Result<ChainSpec, EnvVarReadError> {
	let runtime_genesis_config = RuntimeGenesisConfig {
		system: SystemConfig { ..Default::default() },
		balances: BalancesConfig {
			// Update if any endowed accounts are required.
			balances: vec![],
		},
		aura: AuraConfig { authorities: vec![] },
		grandpa: GrandpaConfig { authorities: vec![], ..Default::default() },
		sudo: SudoConfig {
			// No sudo account by default, please update with your preferences.
			key: None,
		},
		transaction_payment: Default::default(),
		session: SessionConfig {
			// Initial validators are meant to be updated in the chain spec file, so it is empty here.
			initial_validators: vec![],
		},
		sidechain: SidechainConfig {
			params: SidechainParams {
				chain_id: from_var("CHAIN_ID")?,
				governance_authority: from_var::<MainchainAddressHash>("GOVERNANCE_AUTHORITY")?,
				threshold_numerator: from_var("THRESHOLD_NUMERATOR")?,
				threshold_denominator: from_var("THRESHOLD_DENOMINATOR")?,
				genesis_committee_utxo: from_var::<UtxoId>("GENESIS_COMMITTEE_UTXO")?,
			},
			..Default::default()
		},
		polkadot_session_stub_for_grandpa: Default::default(),
		session_committee_management: SessionCommitteeManagementConfig {
			// Same as SessionConfig
			initial_authorities: vec![],
			main_chain_scripts: read_mainchain_scripts_from_env()?,
		},
		native_token_management: NativeTokenManagementConfig {
			main_chain_scripts: read_native_token_main_chain_scripts_from_env()?,
			..Default::default()
		},
	};
	let genesis_json = serde_json::to_value(runtime_genesis_config)
		.expect("Genesis config must be serialized correctly");
	Ok(ChainSpec::builder(runtime_wasm(), None)
		.with_name("Partner Chains Template")
		.with_id("partner_chains_template")
		.with_chain_type(ChainType::Live)
		.with_genesis_config(genesis_json)
		.build())
}
