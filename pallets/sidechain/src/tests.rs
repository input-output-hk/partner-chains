use frame_support::traits::Hooks;

use crate::mock::*;

#[test]
fn on_new_epoch_is_triggered_by_epoch_change() {
	new_test_ext().execute_with(|| {
		Mock::set_slot(4);
		Sidechain::on_initialize(1);
		assert_eq!(mock_pallet::OnNewEpochCallCount::<Test>::get(), 0);

		Mock::set_slot(MOCK_SLOTS_PER_EPOCH.0.into());
		Sidechain::on_initialize(3);
		assert_eq!(mock_pallet::OnNewEpochCallCount::<Test>::get(), 1);
	})
}

#[test]
fn on_new_epoch_is_not_triggered_without_epoch_change() {
	new_test_ext().execute_with(|| {
		Mock::set_slot(1);
		Sidechain::on_initialize(1);
		Sidechain::on_initialize(2);
		Mock::set_slot(u64::from(MOCK_SLOTS_PER_EPOCH.0) - 1);
		Sidechain::on_initialize(3);
		assert_eq!(mock_pallet::OnNewEpochCallCount::<Test>::get(), 0);
	})
}

#[test]
fn read_sidechain_params() {
	new_test_ext().execute_with(|| {
		let params = Sidechain::sidechain_params();
		assert_eq!(params, crate::mock::MOCK_SIDECHAIN_PARAMS);
	})
}
