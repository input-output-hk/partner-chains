use super::*;
use frame_support::traits::Hooks;
use mock::*;

fn make_id(i: u64) -> BlockProducerId {
	let mut id = [0u8; 32];
	id[0..8].copy_from_slice(&i.to_le_bytes());
	id
}

#[test]
fn on_initialize_appends_block_author() {
	new_test_ext().execute_with(|| {
		assert_eq!(Log::<Test>::get(), vec![]);
		Mock::set_block_author(make_id(42));
		Mock::set_moment(1337);
		BlockProductionLog::on_initialize(1);
		assert_eq!(Log::<Test>::get(), vec![(1337, make_id(42))]);
	});
}

#[test]
fn take_prefix() {
	new_test_ext().execute_with(|| {
		Log::<Test>::put(vec![
			(100, make_id(1)),
			(101, make_id(2)),
			(102, make_id(1)),
			(105, make_id(2)),
			(107, make_id(1)),
		]);

		let prefix = BlockProductionLog::take_prefix(&104);
		let left_in_storage = Log::<Test>::get().to_vec();

		assert_eq!(prefix.to_vec(), vec![(100, make_id(1)), (101, make_id(2)), (102, make_id(1)),]);
		assert_eq!(left_in_storage.to_vec(), vec![(105, make_id(2)), (107, make_id(1)),]);
	})
}

#[test]
fn take_prefix_when_there_are_two_entries_for_the_same_moment() {
	new_test_ext().execute_with(|| {
		Log::<Test>::put(vec![
			(100, make_id(1)),
			(104, make_id(2)),
			(104, make_id(1)),
			(105, make_id(2)),
			(107, make_id(1)),
		]);

		let prefix = BlockProductionLog::take_prefix(&104);
		let left_in_storage = Log::<Test>::get().to_vec();

		assert_eq!(prefix.to_vec(), vec![(100, make_id(1)), (104, make_id(2)), (104, make_id(1)),]);
		assert_eq!(left_in_storage.to_vec(), vec![(105, make_id(2)), (107, make_id(1)),]);
	})
}

#[test]
fn drop_prefix() {
	new_test_ext().execute_with(|| {
		Log::<Test>::put(vec![
			(100, make_id(0)),
			(101, make_id(1)),
			(102, make_id(2)),
			(103, make_id(3)),
			(104, make_id(4)),
		]);

		BlockProductionLog::drop_prefix(&102);

		let left_in_storage = Log::<Test>::get().to_vec();

		assert_eq!(left_in_storage.to_vec(), vec![(103, make_id(3)), (104, make_id(4)),]);
	})
}

#[test]
fn peek_prefix() {
	new_test_ext().execute_with(|| {
		Log::<Test>::put(vec![
			(100, make_id(0)),
			(101, make_id(1)),
			(102, make_id(2)),
			(103, make_id(3)),
			(104, make_id(4)),
		]);

		let prefix = BlockProductionLog::peek_prefix(&102u64);

		assert_eq!(
			prefix.collect::<Vec<_>>(),
			vec![(100, make_id(0)), (101, make_id(1)), (102, make_id(2)),]
		);
	})
}
