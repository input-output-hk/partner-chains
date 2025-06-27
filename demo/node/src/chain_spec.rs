use partner_chains_demo_runtime::{
	AccountId, CrossChainPublic, Signature, WASM_BINARY, opaque::SessionKeys,
	config::PartnerChainsConfig,
};
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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

/// Load partner chains configuration from environment or use defaults
pub fn load_partner_chains_config() -> PartnerChainsConfig {
	PartnerChainsConfig::from_env_or_default()
}

/// Create development chain spec with custom partner chains config
pub fn development_config_with_partner_chains_config() -> ChainSpec {
	let config = load_partner_chains_config();
	ChainSpec::builder(
		runtime_wasm(),
		Default::default(),
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(sc_service::ChainType::Development)
	.with_genesis_config_patch(
		partner_chains_demo_runtime::genesis_config_presets::development_config_genesis_with_config(config)
	)
	.build()
}

/// Create local testnet chain spec with custom partner chains config
pub fn local_testnet_config_with_partner_chains_config() -> ChainSpec {
	let config = load_partner_chains_config();
	ChainSpec::builder(
		runtime_wasm(),
		Default::default(),
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(sc_service::ChainType::Local)
	.with_genesis_config_patch(
		partner_chains_demo_runtime::genesis_config_presets::local_config_genesis_with_config(config)
	)
	.build()
}
