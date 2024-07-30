#[allow(unused_imports)]
use std::sync::Arc;

#[cfg(feature = "block-source")]
pub mod block;
#[cfg(feature = "candidate-source")]
pub mod candidate;

#[allow(unused)]
pub(crate) struct UnimplementedMocks;
