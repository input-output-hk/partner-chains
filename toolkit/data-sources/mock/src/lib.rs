#[allow(unused_imports)]
use std::sync::Arc;

#[cfg(feature = "block-source")]
pub mod block;
#[cfg(feature = "candidate-source")]
pub mod candidate;
#[cfg(feature = "governed-map")]
pub mod governed_map;
#[cfg(feature = "mc-hash")]
pub mod mc_hash;
#[cfg(feature = "native-token")]
pub mod native_token;
#[cfg(feature = "sidechain-rpc")]
pub mod sidechain_rpc;
#[cfg(feature = "block-participation")]
pub mod stake_distribution;

#[allow(unused)]
pub(crate) struct UnimplementedMocks;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
