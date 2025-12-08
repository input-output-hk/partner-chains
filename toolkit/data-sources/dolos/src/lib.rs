#[cfg(feature = "candidate-source")]
mod candidate;
#[cfg(feature = "candidate-source")]
pub use candidate::AuthoritySelectionDataSourceImpl;

#[cfg(feature = "governed-map")]
mod governed_map;
#[cfg(feature = "governed-map")]
pub use governed_map::GovernedMapDataSourceImpl;

#[cfg(feature = "mc-hash")]
mod mc_hash;
#[cfg(feature = "mc-hash")]
pub use mc_hash::McHashDataSourceImpl;

#[cfg(feature = "sidechain-rpc")]
mod sidechain_rpc;
#[cfg(feature = "sidechain-rpc")]
pub use sidechain_rpc::SidechainRpcDataSourceImpl;

#[cfg(feature = "block-participation")]
mod stake_distribution;
#[cfg(feature = "block-participation")]
pub use stake_distribution::StakeDistributionDataSourceImpl;

#[cfg(feature = "bridge")]
mod bridge;
#[cfg(feature = "bridge")]
pub use bridge::TokenBridgeDataSourceImpl;

#[cfg(feature = "block-source")]
mod block;
#[cfg(feature = "block-source")]
pub use block::BlockDataSourceImpl;

#[cfg(feature = "block-source")]
use sidechain_domain::mainchain_epoch::MainchainEpochConfig;

use crate::client::MiniBFClient;

pub mod client;

type ResultErr = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, ResultErr>;

/// Error type returned by Dolos based data sources
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum DataSourceError {
	/// Indicates that Dolos rejected a request as invalid
	#[error("Bad request: `{0}`.")]
	BadRequest(String),
	/// Indicates that Dolos client produced an error while calling endpoint
	#[error("Dolos client call error: `{0}`.")]
	DolosCallError(String),
	/// Indicates that Dolos client produced an error while parsing response
	#[error("Dolos client response parse error: `{0}`.")]
	DolosResponseParseError(String),
	/// Indicates that expected data was not found when querying Dolos
	#[error(
		"'{0}' not found. Possible causes: data source configuration error, Dolos not synced fully, or data not set on the main chain."
	)]
	ExpectedDataNotFound(String),
	/// Indicates that data returned by Dolos is invalid
	#[error(
		"Invalid data. {0} Possible cause is an error in Plutus scripts or data source is outdated."
	)]
	InvalidData(String),
}

/// Returns a [MiniBFClient] constructed using configuration read from environment
///
/// # Environment variables read:
/// - `DOLOS_MINIBF_URL`: Dolos MiniBF client, eg. `localhost:3000`
pub fn get_connection_from_env() -> Result<MiniBFClient> {
	log::warn!("Dolos data sources are still WIP and should not be used in production");
	let config = ConnectionConfig::from_env()?;
	Ok(MiniBFClient::new(config.dolos_minibf_url.as_str(), std::time::Duration::from_secs(30)))
}

/// Dolos connection config used when creating a [MiniBFClient].
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ConnectionConfig {
	/// Dolos MiniBF client, eg. `localhost:3000`
	pub(crate) dolos_minibf_url: String,
}

impl ConnectionConfig {
	/// Reads Dolos connection config from the environment
	pub fn from_env() -> Result<Self> {
		let config: Self = figment::Figment::new()
			.merge(figment::providers::Env::raw())
			.extract()
			.map_err(|e| format!("Failed to read Dolos data source connection: {e}"))?;
		Ok(config)
	}
}

/// Reads Cardano main chain epoch configuration from the environment.
///
/// See documentation of [MainchainEpochConfig::read_from_env] for the list of environment variables read.
#[cfg(feature = "block-source")]
pub fn read_mc_epoch_config() -> Result<MainchainEpochConfig> {
	Ok(MainchainEpochConfig::read_from_env()
		.map_err(|e| format!("Failed to read main chain config: {}", e))?)
}
