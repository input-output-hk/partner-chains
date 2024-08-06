use frame_support::{
	pallet_prelude::{InherentData, ProvideInherent},
	traits::Hooks,
};
use sidechain_domain::McBlockHash;

use crate::{mock::*, Call, LastMcHash, Pallet};
use frame_support::traits::UnfilteredDispatchable;
use sidechain_mc_hash::INHERENT_IDENTIFIER;

#[test]
fn on_new_epoch_is_triggered_by_epoch_change() {
	new_test_ext().execute_with(|| {
		Mock::set_slot(4);
		Sidechain::on_initialize(1);
		assert_eq!(Mock::on_new_epoch_call_count(), 0);

		Mock::set_slot(MOCK_SLOTS_PER_EPOCH.0.into());
		Sidechain::on_initialize(3);
		assert_eq!(Mock::on_new_epoch_call_count(), 1);
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
		assert_eq!(Mock::on_new_epoch_call_count(), 0);
	})
}

#[test]
fn read_sidechain_params() {
	new_test_ext().execute_with(|| {
		let params = Sidechain::sidechain_params();
		assert_eq!(params, crate::mock::MOCK_SIDECHAIN_PARAMS);
	})
}

#[test]
fn saves_initial_mc_hash() {
	new_test_ext().execute_with(|| {
		let hash = McBlockHash([123; 32]);
		let call = create_inherent(&hash).unwrap();

		assert!(LastMcHash::<Test>::get().is_none());

		call.dispatch_bypass_filter(RuntimeOrigin::none()).unwrap();

		assert_eq!(LastMcHash::<Test>::get().unwrap(), hash);
	})
}

#[test]
fn skips_inherent_when_mc_hash_unchanged() {
	new_test_ext().execute_with(|| {
		let hash = McBlockHash([123; 32]);
		LastMcHash::<Test>::put(hash.clone());

		let call = create_inherent(&hash);

		assert!(call.is_none());
	})
}

#[test]
fn saves_new_mc_hash() {
	new_test_ext().execute_with(|| {
		let old_hash = McBlockHash([123; 32]);
		let new_hash = McBlockHash([124; 32]);
		LastMcHash::<Test>::put(old_hash);

		let call = create_inherent(&new_hash).unwrap();

		call.dispatch_bypass_filter(RuntimeOrigin::none()).unwrap();

		assert_eq!(LastMcHash::<Test>::get().unwrap(), new_hash);
	})
}

fn create_inherent(mc_hash: &McBlockHash) -> Option<Call<Test>> {
	let mut inherent_data = InherentData::new();
	inherent_data.put_data(INHERENT_IDENTIFIER, mc_hash).unwrap();

	Pallet::<Test>::create_inherent(&inherent_data)
}
