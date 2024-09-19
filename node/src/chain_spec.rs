use sidechain_runtime::{opaque::SessionKeys, AccountId, CrossChainPublic, Signature, WASM_BINARY};
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
