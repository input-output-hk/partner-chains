use crate::mock::*;
use frame_support::{assert_ok, inherent::ProvideInherent, traits::Hooks};

mod inherent_tests {
	use super::*;
	use crate::{CommitteeInfo, Error, pallet};
	use frame_support::assert_err;
	use sp_runtime::DispatchError;

	#[test]
	fn changes_validator_vec_based_on_inherent_data() {
		new_test_ext().execute_with(|| {
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee,
				ids_and_keys_fn(&[ALICE, BOB])
			);
			test_validators_change_through_inherents(&[ALICE]);
		});
	}

	#[test]
	fn prevent_overwriting_committee() {
		new_test_ext().execute_with(|| {
			assert_err!(
				set_validators_directly(&[ALICE, BOB], 0),
				DispatchError::from(Error::<Test>::InvalidEpoch)
			);
			assert_err!(
				set_validators_directly(&[ALICE], 0),
				DispatchError::from(Error::<Test>::InvalidEpoch)
			);
		});
	}

	#[test]
	fn only_one_inherent_can_run_per_block() {
		new_test_ext().execute_with(|| {
			set_validators_directly(&[ALICE, BOB], 1).expect("Frist call should succeed");
			set_validators_directly(&[ALICE, BOB], 1).expect_err("Second call should fail");
		});
	}

	fn test_validators_change_through_inherents(new_validators: &[MockValidator]) {
		set_validators_through_inherents(new_validators);
		let mut expected_validators = ids_and_keys_fn(new_validators);

		expected_validators.sort();

		let mut actual_validators = SessionCommitteeManagement::next_committee_storage()
			.as_ref()
			.map(|c| c.committee.clone())
			.expect("Committee should be set after setting validators through inherents");
		actual_validators.sort();
		assert_eq!(actual_validators, expected_validators);
	}

	#[test]
	fn check_inherent_works() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();
			let validators = vec![ALICE];
			let call = create_inherent_set_validators_call(&validators).expect(
				"create inherent should create a validator set call if given new validators list",
			);
			assert_ok!(<SessionCommitteeManagement as ProvideInherent>::check_inherent(
				&call,
				&create_inherent_data(&validators)
			));
		});
	}

	#[test]
	fn check_inherent_uses_next_committee_if_cannot_calculate_from_data() {
		// This is for the case NextCommittee was set already and committee cannot be selected from inherent data.
		// Block proposer has just rotated NextCommittee to CurrentCommittee and call is to "set" the NextCommittee.
		// Because committee could not be selected from the inherent data, it proposed its CurrentCommittee (after rotation).
		// From block verifier's perspective, it should be its NextCommittee, because it will rotate committee after checking inherents.
		new_test_ext().execute_with(|| {
			// Make sure that the CurrentCommittee is different to the NextCommittee.
			pallet::CurrentCommittee::<Test>::put(CommitteeInfo {
				committee: ids_and_keys_fn(&[CHARLIE, DAVE]),
				epoch: 42,
			});
			pallet::NextCommittee::<Test>::put(CommitteeInfo {
				committee: ids_and_keys_fn(&[ALICE, BOB]),
				epoch: 43,
			});
			let inherent_data_from_which_committee_cannot_be_selected = create_inherent_data(&[]);
			let set_call = pallet::Call::set {
				validators: ids_and_keys_fn(&[ALICE, BOB]),
				for_epoch_number: 43,
			};
			assert_ok!(<SessionCommitteeManagement as ProvideInherent>::check_inherent(
				&set_call,
				&inherent_data_from_which_committee_cannot_be_selected
			));
		});
	}

	#[test]
	fn check_inherent_uses_current_committee_when_cannot_calculate_from_data_and_next_committee_is_not_set()
	 {
		// This case if for `check_inherent` verifying block number 1 when committee cannot be calculated from the inherent data.
		// It is the only situation when NextCommittee is not set.
		let genesis_validators = [ALICE, BOB];
		new_test_ext_with_genesis_initial_authorities(&genesis_validators).execute_with(|| {
			assert!(pallet::NextCommittee::<Test>::get().is_none());
			let inherent_data_from_which_committee_cannot_be_selected = create_inherent_data(&[]);
			let call = pallet::Call::set {
				validators: ids_and_keys_fn(&genesis_validators),
				for_epoch_number: 43,
			};
			// Should be Ok, because check_inherent should use CurrentCommittee value.
			assert_ok!(<SessionCommitteeManagement as ProvideInherent>::check_inherent(
				&call,
				&inherent_data_from_which_committee_cannot_be_selected
			));
		});
	}
}

mod committee_rotation_tests {
	use super::*;
	#[test]
	fn test_current_committee_epoch() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();
			set_validators_through_inherents(&[ALICE]);
			let current_committee_epoch =
				SessionCommitteeManagement::current_committee_storage().epoch;
			assert_eq!(current_committee_epoch, current_epoch_number());
		});
	}

	#[test]
	fn test_next_committee() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();

			let next_committee = SessionCommitteeManagement::next_committee();
			assert_eq!(next_committee, None);

			set_validators_through_inherents(&[ALICE]);

			let next_committee = SessionCommitteeManagement::next_committee();
			assert_eq!(next_committee, Some(authority_ids(&[ALICE])));
		});
	}

	#[test]
	fn test_rotate_committee() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();

			// first committee is Alice and Bob (from the mock)
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				ids_and_keys_fn(&[ALICE, BOB])
			);

			// first committee epoch is the one set in the `initialize_first_committee`
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch_number()
			);

			// verify that the next committee is not set yet
			assert_eq!(SessionCommitteeManagement::next_committee(), None);

			// set the new committee
			set_validators_through_inherents(&[ALICE]);

			// verify that the next committee is set
			assert_eq!(SessionCommitteeManagement::next_committee(), Some(authority_ids(&[ALICE])));

			// increment the epoch (this only changes to mock value for T::current_epoch_number)
			increment_epoch();

			// rotate the committee
			assert_eq!(rotate_committee(), Some(vec![ALICE.authority_id]));

			// verify that the committee storage is set
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				ids_and_keys_fn(&[ALICE])
			);

			// verify that the committee epoch is advanced
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch_number()
			);

			// verify that the next committee is not set
			assert_eq!(SessionCommitteeManagement::next_committee(), None);
		});
	}

	/// Test that in case of a stale committee (where one or more epochs have passed without blocks), the committee rotation
	/// is able to recover by making each missed committee produce a block in their order (a session of length 1 that only
	/// rotates the committee to the next one).
	/// Note that currently this behaviour is more of an example than a real test, because the
	/// source of the next committee is the ariadne_cdp that we can't mock yet.
	#[test]
	fn test_recover_from_stale_committee() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();

			// first committee is Alice and Bob (from the mock)
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				ids_and_keys_fn(&[ALICE, BOB])
			);

			// verify that the next committee is not set yet
			assert_eq!(SessionCommitteeManagement::next_committee(), None);

			// set the new committee
			set_validators_through_inherents(&[ALICE]);

			// 3 epochs pass without any blocks
			increment_epoch();
			increment_epoch();
			increment_epoch();
			let current_epoch = current_epoch_number();

			// Alice and Bob, which were the initial committee, are still the committee (stale)
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				ids_and_keys_fn(&[ALICE, BOB])
			);

			// Alice, which was the next committee, is still the next committee
			assert_eq!(SessionCommitteeManagement::next_committee(), Some(authority_ids(&[ALICE])));
			assert_eq!(
				SessionCommitteeManagement::next_committee_storage().unwrap().epoch,
				current_epoch - 2
			);

			// the first block after 3 epochs rotates to the next committee (Alice)
			assert_eq!(rotate_committee(), Some(vec![ALICE.authority_id]));
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch - 2
			);

			// in the second block the committee  rotates to Charlie
			set_validators_through_inherents(&[CHARLIE]);
			assert_eq!(
				SessionCommitteeManagement::next_committee_storage().unwrap().epoch,
				current_epoch - 1
			);
			assert_eq!(rotate_committee(), Some(vec![CHARLIE.authority_id]));
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch - 1
			);

			// in the third block the committee rotates to Dave
			set_validators_through_inherents(&[DAVE]);
			assert_eq!(
				SessionCommitteeManagement::next_committee_storage().unwrap().epoch,
				current_epoch
			);
			assert_eq!(rotate_committee(), Some(vec![DAVE.authority_id]));
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch
			);

			// in the fourth block the committee sets the next committee to Eve,
			set_validators_through_inherents(&[EVE]);
			// but because we are already caught up the rotation will happen at the end of the epoch

			// verify that the next committee epoch is correct
			assert_eq!(SessionCommitteeManagement::next_committee(), Some(authority_ids(&[EVE])));
		});
	}

	#[test]
	fn store_committee_only_up_to_next_epoch() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();
			set_validators_through_inherents(&[BOB, ALICE]);

			// setting it again should not change the committee
			assert_eq!(create_inherent_set_validators_call(&[ALICE]), None);
		});
	}

	#[test]
	fn do_not_rotate_committee_if_next_committee_is_not_defined() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();

			// verify that the next committee is not set
			assert_eq!(SessionCommitteeManagement::next_committee(), None);

			// rotation should not happen
			assert_eq!(rotate_committee(), None);

			// verify that the committee has not changed
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				ids_and_keys_fn(&[ALICE, BOB])
			);
		});
	}
}

#[test]
fn get_authority_round_robin_works() {
	new_test_ext().execute_with(|| {
		initialize_first_committee();
		set_validators_through_inherents(&[BOB]);
		increment_epoch();
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(0),
			Some(ALICE.ids_and_keys())
		);
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(1),
			Some(BOB.ids_and_keys())
		);
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(2),
			Some(ALICE.ids_and_keys())
		);
		assert!(rotate_committee().is_some());
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(0),
			Some(BOB.ids_and_keys())
		);
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(1),
			Some(BOB.ids_and_keys())
		);
	});
}

fn increment_epoch() {
	mock_pallet::CurrentEpoch::<Test>::put(current_epoch_number() + 1);
}

fn set_epoch(epoch: u64) {
	mock_pallet::CurrentEpoch::<Test>::put(epoch);
}

// in real life first epoch will be something much bigger than 0, that's why it is here
const ARBITRARY_FIRST_EPOCH: u64 = 189374234;
fn initialize_first_committee() {
	set_epoch(ARBITRARY_FIRST_EPOCH);
	SessionCommitteeManagement::on_initialize(1);
}

fn rotate_committee() -> Option<Vec<u64>> {
	let committee = SessionCommitteeManagement::rotate_committee_to_next_epoch()?;
	Some(committee.into_iter().map(|(id, _)| id).collect())
}
