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
		fn should_not_work_if_epoch_does_not_exist(epoch in get_epoch(BEST_NUMBER) + 1..u32::MAX) {
			let client = Arc::new(TestClient { best_number: BEST_NUMBER });

			assert!(client.find_block_number(AnyBlockInEpoch { epoch: ScEpochNumber(epoch as u64) }).is_err());
		}
	}
}
