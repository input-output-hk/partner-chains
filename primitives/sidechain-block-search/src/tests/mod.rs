pub mod conversion;
mod rpc_mock;
mod runtime_api_mock;

use super::*;
use conversion::*;
use proptest::prelude::*;
use rpc_mock::*;
use std::sync::Arc;

pub const BEST_NUMBER: u32 = 100;

mod find_block_number_tests {
	use super::*;

	proptest! {
		#[test]
		fn should_work_if_any_block_in_epoch(epoch in 1u32..get_epoch(BEST_NUMBER)) {
			let client = Arc::new(TestClient { best_number: BEST_NUMBER });

			let block_number = client.find_block_number(AnyBlockInEpoch { epoch: ScEpochNumber(epoch as u64) }).unwrap();
			assert!(get_any_block_in_epoch(epoch).contains(&block_number));
		}

		#[test]
		fn should_work_if_last_block_in_epoch(epoch in 1u32..get_epoch(BEST_NUMBER)) {
			let client = Arc::new(TestClient { best_number: BEST_NUMBER });

			let block_number = client.find_block_number(LastBlockInEpoch { epoch: ScEpochNumber(epoch as u64) }).unwrap();
			assert_eq!(block_number, get_last_block_in_epoch(epoch));
		}

		#[test]
		fn should_work_if_any_block_in_slot_range(start_block in 1u32..BEST_NUMBER, end_block in 1u32..BEST_NUMBER) {
			let client = Arc::new(TestClient { best_number: BEST_NUMBER });

			let slot_range = ScSlotNumber(get_slot(start_block) as u64)..ScSlotNumber(get_slot((start_block + 1) + end_block % (BEST_NUMBER - start_block + 1)) as u64);

			let block_number = client.find_block_number(AnyBlockInSlotRange {
				slot_range: slot_range.clone(),
			}).unwrap();
			assert!((get_block_number(slot_range.start.0 as u32)..get_block_number(slot_range.end.0 as u32)).contains(&block_number));
		}

		#[test]
		fn should_work_if_latest_block_in_slot_range(start_block in 1u32..BEST_NUMBER, end_block in 1u32..BEST_NUMBER) {
			let slot_range = ScSlotNumber(get_slot(start_block) as u64)..ScSlotNumber(get_slot((start_block + 1) + end_block % (BEST_NUMBER - start_block + 1)) as u64);

			for slot_number in slot_range.start.0..slot_range.end.0 {
				let best_block = get_block_number(slot_number as u32);

				let client = Arc::new(TestClient { best_number: best_block });
				let block_number = client.find_block_number(LatestBlockInSlotRange {
					slot_range: slot_range.clone(),
					latest_block: best_block,
				}).unwrap();
				assert_eq!(block_number, best_block);
			}

			let first_block_after_slot_range = get_block_number(slot_range.end.0 as u32);
			let client = Arc::new(TestClient { best_number: first_block_after_slot_range });
			let block_number = client.find_block_number(LatestBlockInSlotRange {
				slot_range: slot_range.clone(),
				latest_block: first_block_after_slot_range,
			}).unwrap();
			assert_eq!(block_number, get_block_number(slot_range.end.0 as u32) - 1);
		}

		#[test]
		fn should_not_work_if_epoch_does_not_exist(epoch in get_epoch(BEST_NUMBER) + 1..u32::MAX) {
			let client = Arc::new(TestClient { best_number: BEST_NUMBER });

			assert!(client.find_block_number(AnyBlockInEpoch { epoch: ScEpochNumber(epoch as u64) }).is_err());
			assert!(client.find_block_number(LastBlockInEpoch { epoch: ScEpochNumber(epoch as u64) }).is_err());
		}
	}
}
