use crate::mock::*;
use crate::*;
use frame_support::assert_ok;
use frame_support::inherent::{InherentData, ProvideInherent};
use frame_system::Origin;
use pretty_assertions::assert_eq;
use sp_block_participation::BlockProductionData;
use sp_block_participation::{INHERENT_IDENTIFIER, Slot};
use sp_runtime::BoundedVec;

#[test]
fn inherent_clears_production_log_prefix() {
	new_test_ext().execute_with(|| {
		let initial_log = (1..=20).map(|i| (Slot::from(i), i + 100)).collect();
		let expected_log: Vec<_> = (11..=20).map(|i| (Slot::from(i), i + 100)).collect();
		mock_pallet::BlockProductionLog::<Test>::put(BoundedVec::truncate_from(initial_log));

		assert_eq!(ProcessedUpToSlot::<Test>::get(), Slot::default());
		Payouts::note_processing(Origin::<Test>::None.into(), Slot::from(10))
			.expect("Should succeed");

		let log = mock_pallet::BlockProductionLog::<Test>::get().unwrap().to_vec();
		assert_eq!(log, expected_log);
		assert_eq!(ProcessedUpToSlot::<Test>::get(), Slot::from(10));
	})
}

#[test]
fn inherent_cant_be_run_twice_in_one_block() {
	new_test_ext().execute_with(|| {
		Payouts::note_processing(Origin::<Test>::None.into(), Slot::from(10))
			.expect("First call should succeed");
		Payouts::note_processing(Origin::<Test>::None.into(), Slot::from(10))
			.expect_err("Second call should fail");
	})
}

#[test]
fn creates_inherent_if_data_present() {
	let mut inherent_data = InherentData::new();
	inherent_data
		.put_data(
			INHERENT_IDENTIFIER,
			&BlockProductionData::<u32, u64>::new(Slot::from(24), vec![]),
		)
		.unwrap();

	let inherent = Payouts::create_inherent(&inherent_data).expect("Should crate an inherent");

	assert_eq!(inherent, crate::Call::<Test>::note_processing { up_to_slot: Slot::from(24) })
}

#[test]
fn check_fails_if_data_is_missing() {
	let inherent_data = InherentData::new();
	let inherent = crate::Call::<Test>::note_processing { up_to_slot: Slot::from(25) };

	let result = Payouts::check_inherent(&inherent, &inherent_data);
	assert_eq!(result, Err(sp_block_participation::InherentError::UnexpectedInherent))
}

#[test]
fn check_fails_if_slot_is_different() {
	let mut inherent_data = InherentData::new();
	inherent_data
		.put_data(
			INHERENT_IDENTIFIER,
			&BlockProductionData::<u32, u64>::new(Slot::from(24), vec![]),
		)
		.unwrap();
	let inherent = crate::Call::<Test>::note_processing { up_to_slot: Slot::from(25) };

	let result = Payouts::check_inherent(&inherent, &inherent_data);
	assert_eq!(result, Err(sp_block_participation::InherentError::IncorrectSlotBoundary))
}

#[test]
fn inherent_is_required_when_data_is_present() {
	let mut inherent_data = InherentData::new();
	inherent_data
		.put_data(
			INHERENT_IDENTIFIER,
			&BlockProductionData::<u32, u64>::new(Slot::from(24), vec![]),
		)
		.unwrap();

	let result = Payouts::is_inherent_required(&inherent_data).unwrap();
	assert_eq!(result, Some(sp_block_participation::InherentError::InherentRequired))
}

#[test]
fn inherent_is_not_required_when_data_is_not_present() {
	let inherent_data = InherentData::new();
	assert_ok!(Payouts::is_inherent_required(&inherent_data), None)
}
