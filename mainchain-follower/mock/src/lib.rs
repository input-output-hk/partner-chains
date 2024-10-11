#[allow(unused_imports)]
use std::sync::Arc;

#[cfg(feature = "block-source")]
pub mod block;
#[cfg(feature = "candidate-source")]
pub mod candidate;
#[cfg(feature = "mc-hash")]
pub mod mc_hash;
#[cfg(feature = "native-token")]
pub mod native_token;

#[allow(unused)]
pub(crate) struct UnimplementedMocks;
