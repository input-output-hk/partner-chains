use crate::mock::*;
use frame_support::{assert_ok, inherent::ProvideInherent, traits::Hooks};
use sidechain_domain::ScEpochNumber;
use sp_session_validator_management::CommitteeMember;

mod inherent_tests {
	use super::*;
	use crate::{CommitteeInfo, Error, pallet};
	use frame_support::assert_err;
	use sidechain_domain::byte_string::SizedByteString;
	use sp_runtime::DispatchError;
	use sp_session_validator_management::InherentError;

	#[test]
	fn changes_validator_vec_based_on_inherent_data() {
		new_test_ext().execute_with(|| {
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee,
				as_permissioned_members(&[alice(), bob()])
			);
			test_validators_change_through_inherents(&[alice()]);
		});
	}

	#[test]
	fn prevent_overwriting_committee() {
		new_test_ext().execute_with(|| {
			assert_err!(
				set_validators_directly(&[alice(), bob()], 0),
				DispatchError::from(Error::<Test>::InvalidEpoch)
			);
			assert_err!(
				set_validators_directly(&[alice()], 0),
				DispatchError::from(Error::<Test>::InvalidEpoch)
			);
		});
	}

	#[test]
	fn only_one_inherent_can_run_per_block() {
		new_test_ext().execute_with(|| {
			set_validators_directly(&[alice(), bob()], 1).expect("Frist call should succeed");
			set_validators_directly(&[alice(), bob()], 1).expect_err("Second call should fail");
		});
	}

	fn test_validators_change_through_inherents(new_validators: &[MockValidator]) {
		set_validators_through_inherents(new_validators);
		let mut expected_validators = as_permissioned_members(new_validators);

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
			let validators = vec![alice()];
			let call = create_inherent_set_validators_call(&validators).expect(
				"create inherent should create a validator set call if given new validators list",
			);
			assert_ok!(<SessionCommitteeManagement as ProvideInherent>::check_inherent(
				&call,
				&create_inherent_data(&validators).0
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
				committee: as_permissioned_members(&[charlie(), dave()]),
				epoch: 42.into(),
			});
			pallet::NextCommittee::<Test>::put(CommitteeInfo {
				committee: as_permissioned_members(&[alice(), bob()]),
				epoch: 43.into(),
			});
			let (inherent_data_from_which_committee_cannot_be_selected, selection_inputs_hash) =
				create_inherent_data(&[]);
			let set_call = pallet::Call::set {
				validators: as_permissioned_members(&[alice(), bob()]),
				for_epoch_number: 43.into(),
				selection_inputs_hash,
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
		let genesis_validators = [alice(), bob()];
		new_test_ext_with_genesis_initial_authorities(&genesis_validators).execute_with(|| {
			assert!(pallet::NextCommittee::<Test>::get().is_none());
			let (inherent_data_from_which_committee_cannot_be_selected, selection_inputs_hash) =
				create_inherent_data(&[]);
			let call = pallet::Call::set {
				validators: as_permissioned_members(&genesis_validators),
				for_epoch_number: 43.into(),
				selection_inputs_hash,
			};
			// Should be Ok, because check_inherent should use CurrentCommittee value.
			assert_ok!(<SessionCommitteeManagement as ProvideInherent>::check_inherent(
				&call,
				&inherent_data_from_which_committee_cannot_be_selected
			));
		});
	}

	#[test]
	fn check_inherent_error_includes_hash_if_correct() {
		let mut genesis_validators = [alice(), bob()];
		new_test_ext_with_genesis_initial_authorities(&genesis_validators).execute_with(|| {
			assert!(pallet::NextCommittee::<Test>::get().is_none());
			let (inherent_data_from_which_committee_cannot_be_selected, selection_inputs_hash) =
				create_inherent_data(&genesis_validators);
			genesis_validators.reverse();
			let call = pallet::Call::set {
				validators: as_permissioned_members(&genesis_validators),
				for_epoch_number: 43.into(),
				selection_inputs_hash: selection_inputs_hash.clone(),
			};

			let expected_err =
				InherentError::InvalidValidatorsMatchingHash(selection_inputs_hash.clone());
			assert_eq!(
				<SessionCommitteeManagement as ProvideInherent>::check_inherent(
					&call,
					&inherent_data_from_which_committee_cannot_be_selected
				),
				Err(expected_err)
			);
		});
	}

	#[test]
	fn check_inherent_error_includes_both_hashes_if_different() {
		let mut genesis_validators = [alice(), bob()];
		new_test_ext_with_genesis_initial_authorities(&genesis_validators).execute_with(|| {
			assert!(pallet::NextCommittee::<Test>::get().is_none());
			let (
				inherent_data_from_which_committee_cannot_be_selected,
				correct_selection_inputs_hash,
			) = create_inherent_data(&genesis_validators);
			let incorrect_selection_inputs_hash = SizedByteString([9u8; 32]);
			genesis_validators.reverse();
			let call = pallet::Call::set {
				validators: as_permissioned_members(&genesis_validators),
				for_epoch_number: 43.into(),
				selection_inputs_hash: incorrect_selection_inputs_hash.clone(),
			};

			let expected_err = InherentError::InvalidValidatorsHashMismatch(
				correct_selection_inputs_hash.clone(),
				incorrect_selection_inputs_hash.clone(),
			);
			assert_eq!(
				<SessionCommitteeManagement as ProvideInherent>::check_inherent(
					&call,
					&inherent_data_from_which_committee_cannot_be_selected
				),
				Err(expected_err)
			);
		});
	}
}

mod committee_rotation_tests {
	use super::*;
	#[test]
	fn test_current_committee_epoch() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();
			set_validators_through_inherents(&[alice()]);
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

			set_validators_through_inherents(&[alice()]);

			let next_committee = SessionCommitteeManagement::next_committee();
			assert_eq!(next_committee, Some(authority_ids(&[alice()])));
		});
	}

	#[test]
	fn test_rotate_committee() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();

			// first committee is Alice and Bob (from the mock)
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				as_permissioned_members(&[alice(), bob()])
			);

			// first committee epoch is the one set in the `initialize_first_committee`
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch_number()
			);

			// verify that the next committee is not set yet
			assert_eq!(SessionCommitteeManagement::next_committee(), None);

			// set the new committee
			set_validators_through_inherents(&[alice()]);

			// verify that the next committee is set
			assert_eq!(
				SessionCommitteeManagement::next_committee(),
				Some(authority_ids(&[alice()]))
			);

			// increment the epoch (this only changes to mock value for T::current_epoch_number)
			increment_epoch();

			// rotate the committee
			assert_eq!(rotate_committee(), Some(vec![alice().authority_id]));

			// verify that the committee storage is set
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				as_permissioned_members(&[alice()])
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
				as_permissioned_members(&[alice(), bob()])
			);

			// verify that the next committee is not set yet
			assert_eq!(SessionCommitteeManagement::next_committee(), None);

			// set the new committee
			set_validators_through_inherents(&[alice()]);

			// 3 epochs pass without any blocks
			increment_epoch();
			increment_epoch();
			increment_epoch();
			let current_epoch = current_epoch_number();

			// Alice and Bob, which were the initial committee, are still the committee (stale)
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().committee.clone(),
				as_permissioned_members(&[alice(), bob()])
			);

			// Alice, which was the next committee, is still the next committee
			assert_eq!(
				SessionCommitteeManagement::next_committee(),
				Some(authority_ids(&[alice()]))
			);
			assert_eq!(
				SessionCommitteeManagement::next_committee_storage().unwrap().epoch,
				current_epoch.saturating_sub(2)
			);

			// the first block after 3 epochs rotates to the next committee (Alice)
			assert_eq!(rotate_committee(), Some(vec![alice().authority_id]));
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch.saturating_sub(2)
			);

			// in the second block the committee  rotates to Charlie
			set_validators_through_inherents(&[charlie()]);
			assert_eq!(
				SessionCommitteeManagement::next_committee_storage().unwrap().epoch,
				current_epoch.saturating_sub(1)
			);
			assert_eq!(rotate_committee(), Some(vec![charlie().authority_id]));
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch.saturating_sub(1)
			);

			// in the third block the committee rotates to Dave
			set_validators_through_inherents(&[dave()]);
			assert_eq!(
				SessionCommitteeManagement::next_committee_storage().unwrap().epoch,
				current_epoch
			);
			assert_eq!(rotate_committee(), Some(vec![dave().authority_id]));
			assert_eq!(
				SessionCommitteeManagement::current_committee_storage().epoch,
				current_epoch
			);

			// in the fourth block the committee sets the next committee to Eve,
			set_validators_through_inherents(&[eve()]);
			// but because we are already caught up the rotation will happen at the end of the epoch

			// verify that the next committee epoch is correct
			assert_eq!(SessionCommitteeManagement::next_committee(), Some(authority_ids(&[eve()])));
		});
	}

	#[test]
	fn store_committee_only_up_to_next_epoch() {
		new_test_ext().execute_with(|| {
			initialize_first_committee();
			set_validators_through_inherents(&[bob(), alice()]);

			// setting it again should not change the committee
			assert_eq!(create_inherent_set_validators_call(&[alice()]), None);
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
				as_permissioned_members(&[alice(), bob()])
			);
		});
	}
}

#[test]
fn get_authority_round_robin_works() {
	new_test_ext().execute_with(|| {
		initialize_first_committee();
		set_validators_through_inherents(&[bob()]);
		increment_epoch();
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(0),
			Some(alice().permissioned())
		);
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(1),
			Some(bob().permissioned())
		);
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(2),
			Some(alice().permissioned())
		);
		assert!(rotate_committee().is_some());
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(0),
			Some(bob().permissioned())
		);
		assert_eq!(
			SessionCommitteeManagement::get_current_authority_round_robin(1),
			Some(bob().permissioned())
		);
	});
}

pub(crate) fn increment_epoch() {
	mock_pallet::CurrentEpoch::<Test>::put(current_epoch_number().next());
}

fn set_epoch(epoch: u64) {
	mock_pallet::CurrentEpoch::<Test>::put(ScEpochNumber(epoch));
}

// in real life first epoch will be something much bigger than 0, that's why it is here
const ARBITRARY_FIRST_EPOCH: u64 = 189374234;
fn initialize_first_committee() {
	set_epoch(ARBITRARY_FIRST_EPOCH);
	SessionCommitteeManagement::on_initialize(1);
}

fn rotate_committee() -> Option<Vec<AuthorityId>> {
	let committee = SessionCommitteeManagement::rotate_committee_to_next_epoch()?;
	Some(committee.iter().map(CommitteeMember::authority_id).collect())
}
