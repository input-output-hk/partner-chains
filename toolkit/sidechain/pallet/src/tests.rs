use frame_support::traits::Hooks;

use crate::mock::*;

#[test]
fn on_new_epoch_is_triggered_by_epoch_change() {
	new_test_ext().execute_with(|| {
		Mock::set_epoch(0);
		Sidechain::on_initialize(1);
		assert_eq!(mock_pallet::OnNewEpochCallCount::<Test>::get(), 0);

		Mock::set_epoch(1);
		Sidechain::on_initialize(3);
		assert_eq!(mock_pallet::OnNewEpochCallCount::<Test>::get(), 1);
	})
}

#[test]
fn on_new_epoch_is_not_triggered_without_epoch_change() {
	new_test_ext().execute_with(|| {
		Mock::set_epoch(0);
		Sidechain::on_initialize(1);
		Sidechain::on_initialize(2);
		Sidechain::on_initialize(3);
		assert_eq!(mock_pallet::OnNewEpochCallCount::<Test>::get(), 0);
	})
}

#[test]
fn read_genesis_utxo() {
	new_test_ext().execute_with(|| {
		let params = Sidechain::genesis_utxo();
		assert_eq!(params, crate::mock::MOCK_GENESIS_UTXO);
	})
}
