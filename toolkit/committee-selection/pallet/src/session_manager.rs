//! Implements [pallet_partner_chains_session::SessionManager] and [pallet_partner_chains_session::ShouldEndSession],
//! Partner Chain's version of Substrate's [pallet_session].
//!
//! To wire the [pallet_partner_chains_session] pallet, a stub version of [pallet_session] has to be wired first.
//! The config for this should be generated with [a macro](pallet_partner_chains_session::impl_pallet_session_config) provided by this crate:
//! ```rust, ignore
//! pallet_partner_chains_session::impl_pallet_session_config!(Runtime);
//! ```
//! which expands to:
//! ```rust, ignore
//! impl pallet_session::Config for Runtime where Runtime: pallet_partner_chains_session::Config { /* ... */ }
//! ```
//!
//! The partner chains session pallet has to be configured, for example:
//! ```rust, ignore
//! impl pallet_partner_chains_session::Config for Runtime {
//! 	type RuntimeEvent = RuntimeEvent;
//! 	type ValidatorId = <Self as frame_system::Config>::AccountId;
//! 	type ShouldEndSession = ValidatorManagementSessionManager<Runtime>;
//! 	type NextSessionRotation = ();
//! 	type SessionManager = ValidatorManagementSessionManager<Runtime>;
//! 	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
//! 	type Keys = opaque::SessionKeys;
//! }
//! ```
//!
//! The order matters when wiring the pallets into the runtime!
//! Partner Chains session_manager [ValidatorManagementSessionManager] writes to [pallet_session::pallet::CurrentIndex].
//! [pallet_partner_chains_session] needs to come last for correct initialization order.
//! [ValidatorManagementSessionManager] is wired in by [pallet_partner_chains_session].
//!
//! ```rust, ignore
//! construct_runtime!(
//! 	pub struct Runtime {
//! 		// ...
//! 		SubstrateSession: pallet_session,
//! 		PcSession: pallet_partner_chains_session,
//! 		// ...
//! 	}
//! );
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

use crate::CommitteeMember;
use core::marker::PhantomData;
use derive_new::new;
use frame_system::pallet_prelude::BlockNumberFor;
use log::info;
use pallet_partner_chains_session::SessionIndex;
use sp_std::vec::Vec;

#[derive(new)]
/// Session manager which takes committee from pallet_session_validator_management.
pub struct ValidatorManagementSessionManager<T> {
	_phantom: PhantomData<T>,
}

impl<T: crate::Config + pallet_session::Config>
	pallet_partner_chains_session::SessionManager<T::AccountId, T::AuthorityKeys>
	for ValidatorManagementSessionManager<T>
{
	fn new_session_genesis(
		_new_index: SessionIndex,
	) -> Option<Vec<(T::AccountId, T::AuthorityKeys)>> {
		Some(
			crate::Pallet::<T>::current_committee_storage()
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
			crate::Pallet::<T>::rotate_committee_to_next_epoch()
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
	T: crate::Config<ScEpochNumber = ScEpochNumber>,
	ScEpochNumber: Clone + PartialOrd,
{
	fn should_end_session(_n: BlockNumberFor<T>) -> bool {
		let current_epoch_number = T::current_epoch_number();

		current_epoch_number > crate::Pallet::<T>::current_committee_storage().epoch
			&& crate::Pallet::<T>::next_committee().is_some()
	}
}

#[cfg(test)]
mod tests {
	use crate::mock::mock_pallet::CurrentEpoch;
	use crate::mock::*;
	use crate::session_manager::*;
	use crate::*;
	use pallet_partner_chains_session::ShouldEndSession;
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
