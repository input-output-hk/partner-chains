//! Implements [pallet_session::SessionManager] and [pallet_session::ShouldEndSession] for [crate::Pallet].
//!
//! This implementation has lag of one additional PC epoch when applying committees to sessions.
//!
//! To use it, wire [crate::Pallet] in runtime configuration of [`pallet_session`].
use crate::{CommitteeRotationStage, CommitteeRotationStages};
use frame_support::traits::UnfilteredDispatchable;
use frame_system::RawOrigin;
use frame_system::pallet_prelude::BlockNumberFor;
use log::{debug, info, warn};
use sp_staking::SessionIndex;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::{vec, vec::Vec};

impl<T: crate::Config + pallet_session::Config> pallet_session::SessionManager<T::AccountId>
	for crate::Pallet<T>
where
	<T as pallet_session::Config>::Keys: From<T::AuthorityKeys>,
{
	/// Sets the first validator-set by mapping the current committee from [crate::Pallet]
	fn new_session_genesis(_new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
		Some(
			crate::Pallet::<T>::current_committee_storage()
				.committee
				.into_iter()
				.map(|member| member.authority_id().into())
				.collect::<Vec<_>>(),
		)
	}

	/// Rotates the committee in [crate::Pallet] and plans this new committee as upcoming validator-set.
	/// Updates the session index of [`pallet_session`].
	// Instead of Some((*).expect) we could just use (*). However, we rather panic in presence of important programming errors.
	fn new_session(new_index: SessionIndex) -> Option<Vec<T::AccountId>> {
		if CommitteeRotationStage::<T>::get() == CommitteeRotationStages::AdditionalSession {
			info!("💼 Session manager: new additional session {new_index}");
			CommitteeRotationStage::<T>::put(CommitteeRotationStages::AwaitEpochChange);
			let committee = crate::Pallet::<T>::current_committee_storage().committee;
			return Some(committee.iter().map(|member| member.authority_id().into()).collect());
		}

		info!("💼 Session manager: new_session {new_index}, rotating the committee");
		let new_committee = crate::Pallet::<T>::rotate_committee_to_next_epoch().expect(
			"Session should never end without current epoch validators defined. \
				Check ShouldEndSession implementation or if it is used before starting new session",
		);

		let old_committee_accounts = crate::ProvidedAccounts::<T>::take();
		let mut new_committee_accounts: BTreeSet<T::AccountId> = BTreeSet::new();

		for member in new_committee.iter() {
			let account: T::AccountId = member.authority_id().into();

			if !new_committee_accounts.contains(&account) {
				new_committee_accounts.insert(account.clone());

				// Members that were already in the old committee have their accounts and keys set up already
				if !old_committee_accounts.contains(&account) {
					provide_account::<T>(&account);
				}

				let new_keys = member.authority_keys();
				let current_keys = load_keys::<T>(&account);

				if current_keys != Some(new_keys.clone().into()) {
					purge_keys::<T>(&account);
					set_keys::<T>(&account, new_keys.clone());
				}
			}
		}

		for account in old_committee_accounts.difference(&new_committee_accounts) {
			purge_keys::<T>(account);
			unprovide_account::<T>(account);
		}

		crate::ProvidedAccounts::<T>::set(new_committee_accounts.clone().try_into().unwrap());

		Some(new_committee.into_iter().map(|member| member.authority_id().into()).collect())
	}

	fn end_session(end_index: SessionIndex) {
		debug!("Session manager: End session {end_index}");
	}

	// Session is expected to be at least 1 block behind sidechain epoch.
	fn start_session(start_index: SessionIndex) {
		let epoch_number = T::current_epoch_number();
		debug!("Session manager: Start session {start_index}, epoch {epoch_number}");
	}
}

fn load_keys<T: pallet_session::Config>(account: &T::AccountId) -> Option<T::Keys> {
	<T as pallet_session::Config>::ValidatorId::try_from(account.clone())
		.ok()
		.as_ref()
		.and_then(pallet_session::Pallet::<T>::load_keys)
}

fn set_keys<T: crate::Config + pallet_session::Config>(
	account: &T::AccountId,
	keys: T::AuthorityKeys,
) where
	<T as pallet_session::Config>::Keys: From<T::AuthorityKeys>,
{
	let set_keys_result = pallet_session::Call::<T>::set_keys { keys: keys.into(), proof: vec![] }
		.dispatch_bypass_filter(RawOrigin::Signed(account.clone()).into());

	match set_keys_result {
		Ok(_) => debug!("set_keys for {account:?}"),
		Err(e) => {
			info!("Could not set_keys for {account:?}, error: {:?}", e.error)
		},
	}
}

fn purge_keys<T: crate::Config + pallet_session::Config>(account: &T::AccountId)
where
	<T as pallet_session::Config>::Keys: From<T::AuthorityKeys>,
{
	let purge_keys_result = pallet_session::Call::<T>::purge_keys {}
		.dispatch_bypass_filter(RawOrigin::Signed(account.clone()).into());
	match purge_keys_result {
		Ok(_) => debug!("purge_keys for {account:?}"),
		Err(e) => info!("Could not purge_keys for {account:?}, error: {:?}", e.error),
	}
}

fn provide_account<T: crate::Config + pallet_session::Config>(account: &T::AccountId) {
	log::debug!(
		"➕💼 Incrementing provider count and registering keys for block producer {account:?}"
	);

	frame_system::Pallet::<T>::inc_providers(&account);
}

fn unprovide_account<T: crate::Config + pallet_session::Config>(account: &T::AccountId) {
	log::info!(
		"➖💼 Decrementing provider count and deregisteringkeys for block producer {account:?}"
	);
	frame_system::Pallet::<T>::dec_providers(&account).expect(
		"We always match dec_providers with corresponding inc_providers, thus it cannot fail",
	);
}

/// Tries to end each session in the first block of each partner chains epoch in which the committee for the epoch is defined.
impl<T> pallet_session::ShouldEndSession<BlockNumberFor<T>> for crate::Pallet<T>
where
	T: crate::Config,
{
	fn should_end_session(n: BlockNumberFor<T>) -> bool {
		let current_epoch_number = T::current_epoch_number();
		let current_committee_epoch = crate::Pallet::<T>::current_committee_storage().epoch;
		let next_committee_is_defined = crate::Pallet::<T>::next_committee().is_some();
		if current_epoch_number > current_committee_epoch {
			if next_committee_is_defined {
				info!("Session manager: should_end_session({n:?}) = true");
				CommitteeRotationStage::<T>::put(CommitteeRotationStages::NewSessionDueEpochChange);
				true
			} else {
				warn!(
					"Session manager: should_end_session({n:?}) 'current epoch' > 'committee epoch' but the next committee is not defined"
				);
				false
			}
		} else {
			let stage = CommitteeRotationStage::<T>::get();
			if stage == CommitteeRotationStages::NewSessionDueEpochChange {
				CommitteeRotationStage::<T>::put(CommitteeRotationStages::AdditionalSession);
				info!("Session manager: should_end_session({n:?}) to force the new committee");
				true
			} else {
				false
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		CommitteeInfo, CurrentCommittee, NextCommittee,
		mock::{mock_pallet::CurrentEpoch, *},
		tests::increment_epoch,
	};
	use pallet_session::ShouldEndSession;
	pub const IRRELEVANT: u64 = 2;
	use sidechain_domain::ScEpochNumber;
	use sp_runtime::testing::UintAuthorityId;

	type Manager = crate::Pallet<Test>;

	#[test]
	fn should_end_session_if_last_one_ended_late_and_new_committee_is_defined() {
		let current_committee_epoch = ScEpochNumber(100);
		let current_committee = as_permissioned_members(&[alice()]);
		let next_committee_epoch = 102;
		let next_committee = as_permissioned_members(&[bob()]);

		new_test_ext().execute_with(|| {
			CurrentCommittee::<Test>::put(CommitteeInfo {
				epoch: current_committee_epoch.into(),
				committee: current_committee,
			});
			CurrentEpoch::<Test>::set(current_committee_epoch.next().next());
			assert!(!Manager::should_end_session(IRRELEVANT));
			NextCommittee::<Test>::put(CommitteeInfo {
				epoch: next_committee_epoch.into(),
				committee: next_committee,
			});
			assert!(Manager::should_end_session(IRRELEVANT));
		});
	}

	#[test]
	fn register_session_keys_for_provided_authorities() {
		new_test_ext().execute_with(|| {
			set_validators_directly(&[dave(), eve()], 1).unwrap();
			// By default, the session keys are not set for the account.
			assert_eq!(Session::load_keys(&dave().account_id()), None);
			assert_eq!(Session::load_keys(&eve().account_id()), None);
			increment_epoch();

			start_session(1);

			// After setting the keys, they should be stored in the session.
			assert_eq!(Session::load_keys(&dave().account_id()), Some(dave().authority_keys));
			assert_eq!(Session::load_keys(&eve().account_id()), Some(eve().authority_keys));
		});
	}

	#[test]
	fn reregister_changed_session_keys_for_sitting_authority() {
		new_test_ext().execute_with(|| {
			set_validators_directly(&[dave(), eve()], 1).unwrap();
			// By default, the session keys are not set for the account.
			assert_eq!(Session::load_keys(&dave().account_id()), None);
			assert_eq!(Session::load_keys(&eve().account_id()), None);
			increment_epoch();

			start_session(1);

			// After setting the keys, they should be stored in the session.
			assert_eq!(Session::load_keys(&dave().account_id()), Some(dave().authority_keys));
			assert_eq!(Session::load_keys(&eve().account_id()), Some(eve().authority_keys));

			let eve_with_different_keys = MockValidator {
				name: "Eve with different keys",
				authority_keys: SessionKeys { foo: UintAuthorityId(44) },
				..eve()
			};

			// Eve is re-elected to the committee, but is using different keys now
			set_validators_directly(&[dave(), eve_with_different_keys.clone()], 2).unwrap();
			increment_epoch();
			start_session(2);

			// Eve's keys were updated
			assert_eq!(
				Session::load_keys(&eve().account_id()),
				Some(eve_with_different_keys.authority_keys)
			);
		});
	}

	#[test]
	fn ends_two_sessions_and_rotates_once_when_committee_changes() {
		new_test_ext().execute_with(|| {
			assert_eq!(Session::current_index(), 0);
			assert_eq!(SessionCommitteeManagement::current_committee_storage().epoch.0, 0);
			increment_epoch();
			set_validators_directly(&[charlie(), dave()], 1).unwrap();

			advance_one_block();
			assert_eq!(Session::current_index(), 1);
			// pallet_session needs additional session to apply charlie() and dave() as validators
			assert_eq!(Session::validators(), vec![]);
			assert_eq!(SessionCommitteeManagement::current_committee_storage().epoch.0, 1);

			advance_one_block();
			assert_eq!(Session::current_index(), 2);
			assert_eq!(Session::validators(), vec![charlie().account_id(), dave().account_id()]);
			assert_eq!(SessionCommitteeManagement::current_committee_storage().epoch.0, 1);

			for _i in 0..10 {
				advance_one_block();
				assert_eq!(Session::current_index(), 2);
				assert_eq!(
					Session::validators(),
					vec![charlie().account_id(), dave().account_id()]
				);
				assert_eq!(SessionCommitteeManagement::current_committee_storage().epoch.0, 1);
			}
		});
	}
}
