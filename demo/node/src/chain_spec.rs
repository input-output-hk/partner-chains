use partner_chains_cli::CreateChainSpecConfig;
use partner_chains_demo_runtime::{
	AccountId, CrossChainPublic, Signature, WASM_BINARY, opaque::SessionKeys,
};
use sc_service::ChainType;
use sidechain_slots::SlotsPerEpoch;
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

pub type ChainSpec = sc_service::GenericChainSpec;

#[derive(Clone, Debug, PartialEq)]
pub struct AuthorityKeys {
	pub session: SessionKeys,
	pub cross_chain: CrossChainPublic,
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(seed, None)
		.expect("static values are valid; qed")
		.public()
}

type AccountPublic = <Signature as Verify>::Signer;

pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

pub fn runtime_wasm() -> &'static [u8] {
	WASM_BINARY.expect("Runtime wasm not available")
}

/// Creates chain-spec according to the config obtained by wizards.
/// [serde_json::Value] is returned instead of [sc_service::GenericChainSpec] in order to avoid
/// GPL code in the toolkit.
pub fn pc_create_chain_spec(config: &CreateChainSpecConfig<SessionKeys>) -> serde_json::Value {
	let runtime_genesis_config = partner_chains_demo_runtime::RuntimeGenesisConfig {
		system: partner_chains_demo_runtime::SystemConfig::default(),
		balances: partner_chains_demo_runtime::BalancesConfig::default(),
		aura: partner_chains_demo_runtime::AuraConfig::default(),
		grandpa: partner_chains_demo_runtime::GrandpaConfig::default(),
		sudo: partner_chains_demo_runtime::SudoConfig::default(),
		transaction_payment: Default::default(),
		session: config.pallet_partner_chains_session_config(),
		sidechain: config.pallet_sidechain_config(SlotsPerEpoch::default()),
		pallet_session: Default::default(),
		session_committee_management: config.pallet_session_validator_management_config(),
		native_token_management: config.native_token_management_config(),
		governed_map: config.governed_map_config(),
		test_helper_pallet: partner_chains_demo_runtime::TestHelperPalletConfig {
			participation_data_release_period: 30,
			..Default::default()
		},
	};
	let genesis_json = serde_json::to_value(runtime_genesis_config)
		.expect("Genesis config must be serialized correctly");
	let chain_spec = ChainSpec::builder(runtime_wasm(), None)
		.with_name("Partner Chains Demo")
		.with_id("partner_chains_demo")
		.with_chain_type(ChainType::Live)
		.with_genesis_config(genesis_json)
		.build();
	let raw = false;
	let chain_spec_str = chain_spec.as_json(raw).expect("Chain spec serialization can not fail");
	serde_json::from_str(&chain_spec_str).unwrap()
}
