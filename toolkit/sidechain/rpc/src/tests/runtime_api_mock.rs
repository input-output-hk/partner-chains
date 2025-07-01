use super::*;
use mock::*;
use rpc_mock::*;
use sidechain_domain::UtxoId;
use sidechain_slots::{ScSlotConfig, SlotsPerEpoch};
use sp_sidechain::GetGenesisUtxo;

sp_api::mock_impl_runtime_apis! {
	impl GetGenesisUtxo<Block> for TestRuntimeApi {
		fn genesis_utxo() -> UtxoId { mock_utxo_id() }
	}

	impl SlotApi<Block> for TestRuntimeApi {
		fn slot_config() -> ScSlotConfig {
			ScSlotConfig {
				slot_duration: self.slot_duration,
				slots_per_epoch: SlotsPerEpoch(self.slots_per_epoch as u32)
			}
		}
	}
}
