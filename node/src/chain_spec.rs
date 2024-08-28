use sidechain_domain::{AssetName, MainchainAddress, PolicyId};
use sidechain_runtime::CrossChainPublic;
use sidechain_runtime::{opaque::SessionKeys, AccountId, Signature, WASM_BINARY};
use sp_core::{Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_session_validator_management::MainChainScripts;
use std::str::FromStr;

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

pub type ChainSpec = sc_service::GenericChainSpec;

pub enum EnvVarReadError {
	Missing(String),
	ParseError(String),
}

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

pub fn from_var_or<T: FromStr>(var: &str, default: T) -> Result<T, EnvVarReadError> {
	if let Ok(env_var_string) = std::env::var(var) {
		env_var_string.parse::<T>().map_err(|_| {
			EnvVarReadError::ParseError(format!(
				"Can not interpret environment variable {}={} as {}",
				var,
				env_var_string,
				std::any::type_name::<T>()
			))
		})
	} else {
		Ok(default)
	}
}

pub fn from_var<T: FromStr>(var: &str) -> Result<T, EnvVarReadError> {
	let env_var_string = std::env::var(var).map_err(|_| {
		EnvVarReadError::Missing(format!("Environment variable {} cannot be empty.", var))
	})?;

	env_var_string.parse::<T>().map_err(|_| {
		EnvVarReadError::ParseError(format!(
			"Can not interpret environment variable {}={} as {}",
			var,
			env_var_string,
			std::any::type_name::<T>(),
		))
	})
}

pub fn read_mainchain_scripts_from_env() -> Result<MainChainScripts, EnvVarReadError> {
	let committee_candidate_address = from_var("COMMITTEE_CANDIDATE_ADDRESS")?;
	let d_parameter_policy = from_var("D_PARAMETER_POLICY_ID")?;
	let permissioned_candidates_policy = from_var::<PolicyId>("PERMISSIONED_CANDIDATES_POLICY_ID")?;
	Ok(MainChainScripts {
		committee_candidate_address,
		d_parameter_policy,
		permissioned_candidates_policy,
	})
}

pub fn read_native_token_main_chain_scripts_from_env(
) -> Result<sp_native_token_management::MainChainScripts, EnvVarReadError> {
	Ok(sp_native_token_management::MainChainScripts {
		native_token_policy: from_var::<PolicyId>("NATIVE_TOKEN_POLICY_ID")?,
		native_token_asset_name: from_var::<AssetName>("NATIVE_TOKEN_ASSET_NAME")?,
		illiquid_supply_address: from_var::<MainchainAddress>("ILLIQUID_SUPPLY_VALIDATOR_ADDRESS")?,
	})
}
