use crate::chain_spec::*;
use partner_chains_demo_runtime::{
	AuraConfig, BalancesConfig, BridgeConfig, GovernedMapConfig, GrandpaConfig,
	NativeTokenManagementConfig, RuntimeGenesisConfig, SessionCommitteeManagementConfig,
	SessionConfig, SidechainConfig, SudoConfig, SystemConfig, TestHelperPalletConfig,
};
use sc_service::ChainType;

/// Produces template chain spec for Partner Chains.
/// This code should be run by `partner-chains-node wizards chain-spec`, to produce JSON chain spec file.
/// `initial_validators` fields should be updated by the `partner-chains-node wizards chain-spec`.
/// Add and modify other fields of `ChainSpec` accordingly to the needs of your chain.
pub fn chain_spec() -> Result<ChainSpec, envy::Error> {
	let runtime_genesis_config = RuntimeGenesisConfig {
		system: SystemConfig { ..Default::default() },
		balances: BalancesConfig {
			// Update if any endowed accounts are required.
			balances: vec![],
			dev_accounts: None,
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
			genesis_utxo: sp_sidechain::read_genesis_utxo_from_env_with_defaults()?,
			..Default::default()
		},
		pallet_session: Default::default(),
		session_committee_management: SessionCommitteeManagementConfig {
			// Same as SessionConfig
			initial_authorities: vec![],
			main_chain_scripts: sp_session_validator_management::MainChainScripts::read_from_env()?,
		},
		native_token_management: NativeTokenManagementConfig {
			main_chain_scripts: Some(sp_native_token_management::MainChainScripts::read_from_env()?),
			..Default::default()
		},
		governed_map: GovernedMapConfig {
			main_chain_scripts: Some(sp_governed_map::MainChainScriptsV1::read_from_env()?),
			..Default::default()
		},
		test_helper_pallet: TestHelperPalletConfig {
			participation_data_release_period: 30,
			..Default::default()
		},
		bridge: BridgeConfig {
			main_chain_scripts: Some(sp_partner_chains_bridge::MainChainScripts::read_from_env()?),
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
