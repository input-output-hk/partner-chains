//! Backward compatibility support for legacy features and APIs

/// Support for legacy chains that used to implement `sidechain_slots::SlotApi`
pub(crate) mod slots {
	use parity_scale_codec::Decode;
	use sp_api::{CallApiAt, CallApiAtParams};
	use sp_runtime::traits::Block as BlockT;

	/// Raw method name of `sidechain_slots::SlotApi::slot_config`
	const RAW_GET_SLOT_RUNTIME_METHOD: &str = "SlotApi_slot_config";

	/// Slot configuration type
	#[derive(sp_core::Decode, Debug)]
	pub(crate) struct ScSlotConfig {
		pub(crate) slots_per_epoch: u32,
		pub(crate) slot_duration_millis: u64,
	}

	/// Trait to read slot config using `sidechain_slots::SlotApi`, failing gracefully if it's not available
	pub(crate) trait GetScSlotConfig<Block: BlockT> {
		fn get_sc_slot_config(&self, best_block: Block::Hash) -> Option<ScSlotConfig>;
	}

	impl<Client, Block> GetScSlotConfig<Block> for Client
	where
		Block: BlockT,
		Client: CallApiAt<Block> + Send + Sync + 'static,
	{
		fn get_sc_slot_config(&self, best_block: Block::Hash) -> Option<ScSlotConfig> {
			let call_params = CallApiAtParams {
				at: best_block,
				function: RAW_GET_SLOT_RUNTIME_METHOD,
				arguments: vec![],
				overlayed_changes: &Default::default(),
				call_context: sp_api::CallContext::Offchain,
				recorder: &None,
				extensions: &Default::default(),
			};
			let raw_result = self.call_api_at(call_params).ok()?;
			let slot_config = ScSlotConfig::decode(&mut &raw_result[..]).ok()?;
			Some(slot_config)
		}
	}
}
