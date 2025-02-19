#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;
use derive_new::new;
use frame_system::pallet_prelude::BlockNumberFor;
use log::info;
use pallet_session_validator_management::CommitteeMember;
use sp_staking::SessionIndex;
use sp_std::vec::Vec;

/// [`pallet_session`] and [`pallet_session_validator_management`] integration.
pub mod pallet_session_support;

#[derive(new)]
pub struct ValidatorManagementSessionManager<T> {
	_phantom: PhantomData<T>,
}

/// SessionManager, which takes committee from pallet_session_validator_management.
impl<T: pallet_session_validator_management::Config + pallet_session::Config>
	pallet_partner_chains_session::SessionManager<T::AccountId, T::AuthorityKeys>
	for ValidatorManagementSessionManager<T>
{
	fn new_session_genesis(
		_new_index: SessionIndex,
	) -> Option<Vec<(T::AccountId, T::AuthorityKeys)>> {
		Some(
			pallet_session_validator_management::Pallet::<T>::current_committee_storage()
				.committee
				.into_iter()
				.map(|member| (member.authority_id().into(), member.authority_keys()))
				.collect::<Vec<_>>(),
		)
	}

	// Instead of Some((*).expect) we could just use (*). However, we rather panic in presence of
	// important programming errors.
	fn new_session(new_index: SessionIndex) -> Option<Vec<(T::AccountId, T::AuthorityKeys)>> {
		info!("New session {new_index}");
		pallet_session::pallet::CurrentIndex::<T>::put(new_index);
		Some(
			pallet_session_validator_management::Pallet::<T>::rotate_committee_to_next_epoch()
				.expect(
					"Session should never end without current epoch validators defined. \
				Check ShouldEndSession implementation or if it is used before starting new session",
				)
				.into_iter()
				.map(|member| (member.authority_id().into(), member.authority_keys()))
				.collect(),
		)
	}

	fn end_session(end_index: SessionIndex) {
		info!("End session {end_index}");
	}

	// Session is expected to be at least 1 block behind sidechain epoch.
	fn start_session(start_index: SessionIndex) {
		let epoch_number = T::current_epoch_number();
		info!("Start session {start_index}, epoch {epoch_number}");
	}
}

/// This implementation tries to end each session in the first block of each sidechain epoch in which
/// the committee for the epoch is defined.
impl<T, ScEpochNumber> pallet_partner_chains_session::ShouldEndSession<BlockNumberFor<T>>
	for ValidatorManagementSessionManager<T>
where
	T: pallet_session_validator_management::Config<ScEpochNumber = ScEpochNumber>,
	ScEpochNumber: Clone + PartialOrd,
{
	fn should_end_session(_n: BlockNumberFor<T>) -> bool {
		let current_epoch_number = T::current_epoch_number();

		current_epoch_number
			> pallet_session_validator_management::Pallet::<T>::current_committee_storage().epoch
			&& pallet_session_validator_management::Pallet::<T>::next_committee().is_some()
	}
}

#[cfg(test)]
mod tests {
	use crate::*;
	use pallet_partner_chains_session::ShouldEndSession;
	use pallet_session_validator_management::mock::mock_pallet::CurrentEpoch;
	use pallet_session_validator_management::mock::*;
	use pallet_session_validator_management::{CommitteeInfo, CurrentCommittee, NextCommittee};
	pub const IRRELEVANT: u64 = 2;

	type Manager = ValidatorManagementSessionManager<Test>;

	#[test]
	fn should_end_session_if_last_one_ended_late_and_new_committee_is_defined() {
		let current_committee_epoch = 100;
		let current_committee = ids_and_keys_fn(&[ALICE]);
		let next_committee_epoch = 102;
		let next_committee = ids_and_keys_fn(&[BOB]);

		new_test_ext().execute_with(|| {
			CurrentCommittee::<Test>::put(CommitteeInfo {
				epoch: current_committee_epoch,
				committee: current_committee,
			});
			CurrentEpoch::<Test>::set(current_committee_epoch + 2);
			assert!(!Manager::should_end_session(IRRELEVANT));
			NextCommittee::<Test>::put(CommitteeInfo {
				epoch: next_committee_epoch,
				committee: next_committee,
			});
			assert!(Manager::should_end_session(IRRELEVANT));
		});
	}
}
