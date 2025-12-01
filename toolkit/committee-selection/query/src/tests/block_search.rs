use super::*;
use proptest::prelude::*;
use std::sync::Arc;

use crate::tests::conversion::*;

mod find_block_number_tests {
	use super::*;

	proptest! {
		#[test]
		fn should_find_any_block_in_epoch(epoch in get_epoch(1u32)..get_epoch(BEST_NUMBER)) {
			let client = Arc::new(TestApi {});

			let block_hash = client.find_any_block_in_epoch(ScEpochNumber(epoch as u64)).unwrap().into();
			let block_number = block_hash_to_block_number(block_hash);

			assert!(epoch_block_range(epoch).contains(&block_number));
		}

		#[test]
		fn should_return_error_when_no_block_in_epoch(epoch in get_epoch(BEST_NUMBER) + 1..u32::MAX) {
			let client = Arc::new(TestApi {});

			assert!(client.find_any_block_in_epoch(ScEpochNumber(epoch as u64)).is_err());
		}
	}
}
