//! Implements Substrate's [pallet_session].
//!
//! This implementation has lag of one additional PC epoch when applying committees to sessions.
use crate::CommitteeMember;
use core::marker::PhantomData;
use derive_new::new;
use frame_support::traits::UnfilteredDispatchable;
use frame_system::RawOrigin;
use frame_system::pallet_prelude::BlockNumberFor;
use log::{debug, info, warn};
use pallet_partner_chains_session::SessionIndex;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::{vec, vec::Vec};

/// Implements [pallet_session::SessionManager] and [pallet_session::ShouldEndSession] integrated with [crate::Pallet].
///
/// To use it, wire it in runtime configuration of [`pallet_session`].
#[allow(dead_code)]
#[derive(new)]
pub struct PalletSessionSupport<T> {
	_phantom: PhantomData<T>,
}

impl<T: crate::Config + pallet_session::Config> pallet_session::SessionManager<T::AccountId>
	for PalletSessionSupport<T>
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
		info!("PalletSessionSupport: new_session {new_index}");
		let new_committee = crate::Pallet::<T>::rotate_committee_to_next_epoch().expect(
			"Session should never end without current epoch validators defined. \
				Check ShouldEndSession implementation or if it is used before starting new session",
		);

		provide_committee_accounts::<T>(&new_committee);
		register_committee_keys::<T>(&new_committee);

		let new_committee_accounts =
			new_committee.into_iter().map(|member| member.authority_id().into()).collect();

		Some(new_committee_accounts)
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

// Registers keys of new committee members in the session pallet. This is necessary, as the pallet
// requires the keys to be registered prior to session start and we do not wish to force block
// producers to do it manually.
fn register_committee_keys<T: crate::Config + pallet_session::Config>(
	new_committee: &[T::CommitteeMember],
) where
	<T as pallet_session::Config>::Keys: From<T::AuthorityKeys>,
{
	let mut keys_added: BTreeSet<T::AccountId> = BTreeSet::new();
	for member in new_committee.iter() {
		let account_id = member.authority_id().into();

		if keys_added.contains(&account_id) {
			continue;
		}

		keys_added.insert(account_id.clone());
		let call = pallet_session::Call::<T>::set_keys {
			keys: From::from(member.authority_keys()),
			proof: vec![],
		};
		let call_result = call.dispatch_bypass_filter(RawOrigin::Signed(account_id.clone()).into());
		match call_result {
			Ok(_) => debug!("set_keys for {account_id:?}"),
			Err(e) => info!("Could not set_keys for {account_id:?}, error: {:?}", e.error),
		}
	}
}

// Ensures that all accounts tied to new committee members exist by incrementing their
// account provider counts. Cleans up accounts of old members by decrementing them back.
fn provide_committee_accounts<T: crate::Config>(new_committee: &[T::CommitteeMember]) {
	let new_accs: BTreeSet<T::AccountId> =
		new_committee.iter().map(|m| m.authority_id().into()).collect();
	let old_accs: BTreeSet<T::AccountId> = crate::Pallet::<T>::current_committee_storage()
		.committee
		.iter()
		.map(|m| m.authority_id().into())
		.collect();

	let to_inc = new_accs.difference(&old_accs);
	let to_dec = old_accs.difference(&new_accs);
	for account in to_inc {
		frame_system::Pallet::<T>::inc_providers(account);
	}
	for account in to_dec {
		frame_system::Pallet::<T>::dec_providers(account).expect(
			"We always match dec_providers with corresponding inc_providers, thus it cannot fail",
		);
	}
}

/// Tries to end each session in the first block of each partner chains epoch in which the committee for the epoch is defined.
impl<T, EpochNumber> pallet_session::ShouldEndSession<BlockNumberFor<T>> for PalletSessionSupport<T>
where
	T: crate::Config<ScEpochNumber = EpochNumber>,
	EpochNumber: Clone + PartialOrd,
{
	fn should_end_session(n: BlockNumberFor<T>) -> bool {
		let current_epoch_number = T::current_epoch_number();
		let current_committee_epoch = crate::Pallet::<T>::current_committee_storage().epoch;
		let next_committee_is_defined = crate::Pallet::<T>::next_committee().is_some();
		if current_epoch_number > current_committee_epoch {
			if next_committee_is_defined {
				debug!("PalletSessionSupport: should_end_session({n:?}) = true");
				true
			} else {
				warn!(
					"PalletSessionSupport: should_end_session({n:?}) 'current epoch' > 'committee epoch' but the next committee is not defined"
				);
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
	use super::*;
	use crate::{
		CommitteeInfo, CurrentCommittee, NextCommittee, mock::mock_pallet::CurrentEpoch, mock::*,
	};
	use pallet_session::ShouldEndSession;
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
