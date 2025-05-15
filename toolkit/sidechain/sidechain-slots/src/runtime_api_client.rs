//! Module providing helper functions for interacting with [SlotApi]

use crate::{ScSlotConfig, SlotApi};
use sp_api::{ApiError, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;

/// Retrieves the slot configuration using runtime API
pub fn slot_config<B, C>(client: &C) -> Result<ScSlotConfig, ApiError>
where
	B: BlockT,
	C: ProvideRuntimeApi<B>,
	C: HeaderBackend<B>,
	C::Api: SlotApi<B>,
{
	client.runtime_api().slot_config(client.info().best_hash)
}
