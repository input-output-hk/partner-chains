use super::*;
use mock::*;
use rpc_mock::*;
use sidechain_domain::UtxoId;
use sidechain_slots::{ScSlotConfig, SlotsPerEpoch};
use sp_sidechain::{GetGenesisUtxo, SidechainStatus};

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

	impl GetSidechainStatus<Block> for TestRuntimeApi {
		#[advanced]
		fn get_sidechain_status(at: <Block as BlockT>::Hash) -> Result<SidechainStatus, sp_api::ApiError> {
			for (hash, status) in self.sidechain_status.iter() {
				if *hash == at {
					return Ok(status.clone())
				}
			}
			panic!("Unexpected get_sidechain_status call for hash {at}");
		}
	}
}
