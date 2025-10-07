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

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
