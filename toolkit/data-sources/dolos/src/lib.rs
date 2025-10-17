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

pub mod client;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Error type returned by Dolos based data sources
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum DataSourceError {
	/// Indicates that Dolos rejected a request as invalid
	#[error("Bad request: `{0}`.")]
	BadRequest(String),
	/// Indicates that an internal error occured when querying Dolos
	#[error("Internal error of data source: `{0}`.")]
	InternalDataSourceError(String),
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
