use super::*;
use frame_support::{assert_err, assert_ok, traits::UnfilteredDispatchable};
use mock::*;
use sp_consensus_slots::Slot;

fn make_id(i: u64) -> [u8; 32] {
	let mut id = [0u8; 32];
	id[0..8].copy_from_slice(&i.to_le_bytes());
	id
}

#[test]
fn first_append_should_succeed() {
	new_test_ext().execute_with(|| {
		let call = Call::<Test>::append { block_producer_id: make_id(1) };
		assert_ok!(call.dispatch_bypass_filter(RuntimeOrigin::none()));

		assert_eq!(Log::<Test>::get().to_vec(), vec![(Slot::from(1001000), make_id(1))]);
	})
}

#[test]
fn append_to_end_of_log() {
	new_test_ext().execute_with(|| {
		Log::<Test>::put(vec![(Slot::from(100), make_id(1))]);
		System::set_block_number(1001);
		LatestBlock::<Test>::put(1000);

		let call = Call::<Test>::append { block_producer_id: make_id(2) };
		assert_ok!(call.dispatch_bypass_filter(RuntimeOrigin::none()));
		assert_eq!(LatestBlock::<Test>::get(), Some(1001));
		assert_eq!(
			Log::<Test>::get().to_vec(),
			vec![(Slot::from(100), make_id(1)), (Slot::from(1001000), make_id(2))]
		);
	})
}

#[test]
fn can_not_append_item_twice_in_the_same_block() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1011);

		let call = Call::<Test>::append { block_producer_id: make_id(2) };
		call.clone().dispatch_bypass_filter(RuntimeOrigin::none()).unwrap();
		assert_err!(
			call.dispatch_bypass_filter(RuntimeOrigin::none()),
			Error::<Test>::BlockNumberNotIncreased
		);
	})
}

#[test]
fn can_not_append_twice_for_same_block_even_after_take_full_prefix() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1001);
		let call = Call::<Test>::append { block_producer_id: make_id(2) };
		call.clone().dispatch_bypass_filter(RuntimeOrigin::none()).unwrap();
		BlockProductionLog::take_prefix(&Slot::from(100));
		assert_err!(
			call.dispatch_bypass_filter(RuntimeOrigin::none()),
			Error::<Test>::BlockNumberNotIncreased
		);
	})
}

#[test]
fn take_prefix() {
	new_test_ext().execute_with(|| {
		Log::<Test>::put(vec![
			(Slot::from(100), make_id(1)),
			(Slot::from(101), make_id(2)),
			(Slot::from(102), make_id(1)),
			(Slot::from(105), make_id(2)),
			(Slot::from(107), make_id(1)),
		]);

		let prefix = BlockProductionLog::take_prefix(&Slot::from(104));
		let left_in_storage = Log::<Test>::get().to_vec();

		assert_eq!(
			prefix.to_vec(),
			vec![
				(Slot::from(100), make_id(1)),
				(Slot::from(101), make_id(2)),
				(Slot::from(102), make_id(1)),
			]
		);
		assert_eq!(
			left_in_storage.to_vec(),
			vec![(Slot::from(105), make_id(2)), (Slot::from(107), make_id(1)),]
		);
	})
}

#[test]
fn take_prefix_when_there_are_two_entries_for_the_same_slot() {
	new_test_ext().execute_with(|| {
		Log::<Test>::put(vec![
			(Slot::from(100), make_id(1)),
			(Slot::from(104), make_id(2)),
			(Slot::from(104), make_id(1)),
			(Slot::from(105), make_id(2)),
			(Slot::from(107), make_id(1)),
		]);

		let prefix = BlockProductionLog::take_prefix(&Slot::from(104));
		let left_in_storage = Log::<Test>::get().to_vec();

		assert_eq!(
			prefix.to_vec(),
			vec![
				(Slot::from(100), make_id(1)),
				(Slot::from(104), make_id(2)),
				(Slot::from(104), make_id(1)),
			]
		);
		assert_eq!(
			left_in_storage.to_vec(),
			vec![(Slot::from(105), make_id(2)), (Slot::from(107), make_id(1)),]
		);
	})
}
