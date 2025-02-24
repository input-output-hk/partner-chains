use core::marker::PhantomData;
use derive_new::new;
use frame_system::pallet_prelude::BlockNumberFor;
use log::{debug, warn};
use pallet_session_validator_management::CommitteeMember;
use sp_staking::SessionIndex;
use sp_std::vec::Vec;

/// Implements [`pallet_session::SessionManager`] and [`pallet_session::ShouldEndSession`] integrated with [`pallet_session_validator_management`].
///
/// To use it, wire it in runtime configuration of [`pallet_session`].
#[allow(dead_code)]
#[derive(new)]
pub struct PalletSessionSupport<T> {
	_phantom: PhantomData<T>,
}

impl<T: pallet_session_validator_management::Config + pallet_session::Config>
	pallet_session::SessionManager<T::AccountId> for PalletSessionSupport<T>
{
	/// Sets the first validator-set by mapping the current committee from [`pallet_session_validator_management`]
	fn new_session_genesis(_new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
		Some(
			pallet_session_validator_management::Pallet::<T>::current_committee_storage()
				.committee
				.into_iter()
				.map(|member| member.authority_id().into())
				.collect::<Vec<_>>(),
		)
	}

	/// Rotates the committee in [`pallet_session_validator_management`] and plans this new committee as upcoming validator-set.
	/// Updates the session index of [`pallet_session`].
	// Instead of Some((*).expect) we could just use (*). However, we rather panic in presence of important programming errors.
	fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
		debug!("PalletSessionSupport: New session {new_index}");
		pallet_session::pallet::CurrentIndex::<T>::put(new_index);
		Some(
			pallet_session_validator_management::Pallet::<T>::rotate_committee_to_next_epoch()
				.expect(
					"PalletSessionSupport: Session should never end without current epoch validators defined. This may be caused by ShouldEndSession invalid behavior or being called before starting new session",
				).into_iter().map(|member| member.authority_id().into()).collect::<Vec<_>>(),
		)
	}

	fn end_session(end_index: SessionIndex) {
		debug!("PalletSessionSupport: End session {end_index}");
	}

	// Session is expected to be at least 1 block behind sidechain epoch.
	fn start_session(start_index: SessionIndex) {
		let epoch_number = T::current_epoch_number();
		debug!("PalletSessionSupport: Start session {start_index}, epoch {epoch_number}");
	}
}

/// Tries to end each session in the first block of each partner chains epoch in which the committee for the epoch is defined.
impl<T, EpochNumber> pallet_session::ShouldEndSession<BlockNumberFor<T>> for PalletSessionSupport<T>
where
	T: pallet_session_validator_management::Config<ScEpochNumber = EpochNumber>,
	EpochNumber: Clone + PartialOrd,
{
	fn should_end_session(n: BlockNumberFor<T>) -> bool {
		let current_epoch_number = T::current_epoch_number();
		let current_committee_epoch =
			pallet_session_validator_management::Pallet::<T>::current_committee_storage().epoch;
		let next_committee_is_defined =
			pallet_session_validator_management::Pallet::<T>::next_committee().is_some();
		if current_epoch_number > current_committee_epoch {
			if next_committee_is_defined {
				debug!("PalletSessionSupport: should_end_session({n:?}) = true");
				true
			} else {
				warn!("PalletSessionSupport: should_end_session({n:?}) 'current epoch' > 'committee epoch' but the next committee is not defined");
				false
			}
		} else {
			debug!("PalletSessionSupport: should_end_session({n:?}) = false");
			false
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::pallet_session_support::PalletSessionSupport;
	use pallet_session::ShouldEndSession;
	use pallet_session_validator_management::{
		mock::mock_pallet::CurrentEpoch, mock::*, CommitteeInfo, CurrentCommittee, NextCommittee,
	};
	pub const IRRELEVANT: u64 = 2;

	type Manager = PalletSessionSupport<Test>;

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
