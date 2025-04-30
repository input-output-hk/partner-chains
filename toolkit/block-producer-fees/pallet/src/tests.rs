use super::*;
use frame_support::{assert_err, assert_ok};
use frame_system::pallet_prelude::OriginFor;
use mock::*;
use sp_consensus_slots::Slot;
use sp_runtime::{AccountId32, DispatchError};
use sp_std::collections::vec_deque::VecDeque;

#[test]
fn stores_the_configured_number_of_fee_changes() {
	new_test_ext().execute_with(|| {
		let alice = AccountId32::new([1u8; 32]);
		let bob = AccountId32::new([2u8; 32]);
		let charlie = AccountId32::new([3u8; 32]);

		mock_pallet::CurrentSlot::<Test>::set(Slot::from(1));
		assert_ok!(Pallet::<Test>::set_fee(OriginFor::<Test>::signed(alice.clone()), 101));
		assert_ok!(Pallet::<Test>::set_fee(OriginFor::<Test>::signed(bob.clone()), 201));

		mock_pallet::CurrentSlot::<Test>::set(Slot::from(2));
		assert_ok!(Pallet::<Test>::set_fee(OriginFor::<Test>::signed(alice.clone()), 102));

		mock_pallet::CurrentSlot::<Test>::set(Slot::from(3));
		// Setting third fee causes drop of the first one
		assert_ok!(Pallet::<Test>::set_fee(OriginFor::<Test>::signed(alice.clone()), 103));

		let alice_entry_2 = (Slot::from(2), 102);
		let alice_entry_3 = (Slot::from(3), 103);
		let alice_entries = VecDeque::from(vec![alice_entry_3, alice_entry_2]);
		let bob_entry_1 = (Slot::from(1), 201);
		let bob_entries = VecDeque::from(vec![bob_entry_1]);

		assert_eq!(Pallet::<Test>::get(alice.clone()), alice_entries);
		assert_eq!(Pallet::<Test>::get_latest(alice.clone()), Some(alice_entry_3));

		assert_eq!(Pallet::<Test>::get(bob.clone()), bob_entries);
		assert_eq!(Pallet::<Test>::get_latest(bob.clone()), Some(bob_entry_1));

		assert_eq!(Pallet::<Test>::get(charlie.clone()), VecDeque::new());
		assert_eq!(Pallet::<Test>::get_latest(charlie.clone()), None);

		assert_eq!(
			Pallet::<Test>::get_all().collect::<Vec<_>>(),
			vec![(alice.clone(), alice_entries), (bob.clone(), bob_entries)]
		);

		assert_eq!(
			Pallet::<Test>::get_all_latest().collect::<Vec<_>>(),
			vec![(alice.clone(), alice_entry_3), (bob.clone(), bob_entry_1)]
		);
	})
}

#[test]
fn accepts_only_signed_origin() {
	new_test_ext().execute_with(|| {
		assert_err!(
			Pallet::<Test>::set_fee(OriginFor::<Test>::none(), 1),
			DispatchError::BadOrigin
		);
		assert_err!(
			Pallet::<Test>::set_fee(OriginFor::<Test>::root(), 1),
			DispatchError::BadOrigin
		);
	})
}

#[test]
fn rejects_fee_over_10000() {
	new_test_ext().execute_with(|| {
		assert_err!(
			Pallet::<Test>::set_fee(OriginFor::<Test>::signed(AccountId32::new([0u8; 32])), 10001),
			DispatchError::Other("fee numerator must be in range from 0 to 10000")
		);
	})
}
