//! Implements storage migration of the `session-validator-management` pallet from v0 to v1.
#[cfg(feature = "try-runtime")]
extern crate alloc;
use frame_support::traits::UncheckedOnRuntimeUpgrade;
#[cfg(feature = "try-runtime")]
use {
	alloc::vec::Vec, parity_scale_codec::Encode, sp_session_validator_management::CommitteeMember,
};

use super::v0;

/// [VersionedMigration] parametrized for v0 to v1 migration.
pub type LegacyToV1Migration<T> = frame_support::migrations::VersionedMigration<
	0, // The migration will only execute when the on-chain storage version is 0
	1, // The on-chain storage version will be set to 1 after the migration is complete
	InnerMigrateV0ToV1<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;

/// Helper type used internally for migration. Use [LegacyToV1Migration] in your runtime instead.
pub struct InnerMigrateV0ToV1<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: crate::pallet::Config> UncheckedOnRuntimeUpgrade for InnerMigrateV0ToV1<T>
where
	T::CommitteeMember: From<(T::AuthorityId, T::AuthorityKeys)>,
{
	fn on_runtime_upgrade() -> sp_runtime::Weight {
		use sp_core::Get;
		use sp_runtime::BoundedVec;

		let current_committee_v0 = v0::CurrentCommittee::<T>::get();
		let current_committee_v1 = crate::pallet::CommitteeInfo::<
			T::ScEpochNumber,
			T::CommitteeMember,
			T::MaxValidators,
		> {
			epoch: current_committee_v0.epoch,
			committee: BoundedVec::truncate_from(
				current_committee_v0.committee.into_iter().map(From::from).collect(),
			),
		};

		crate::CurrentCommittee::<T>::put(current_committee_v1);

		let Some(next_committee_v0) = v0::NextCommittee::<T>::get() else {
			return T::DbWeight::get().reads_writes(2, 1);
		};
		let next_committee_v1 = crate::pallet::CommitteeInfo::<
			T::ScEpochNumber,
			T::CommitteeMember,
			T::MaxValidators,
		> {
			epoch: next_committee_v0.epoch,
			committee: BoundedVec::truncate_from(
				next_committee_v0.committee.into_iter().map(From::from).collect(),
			),
		};

		crate::NextCommittee::<T>::put(next_committee_v1);

		T::DbWeight::get().reads_writes(2, 2)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		let current_committee_v0 = v0::CurrentCommittee::<T>::get();
		let next_committee_v0 = v0::NextCommittee::<T>::get();
		Ok((current_committee_v0, next_committee_v0).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;
		use parity_scale_codec::Decode;
		use v0::LegacyCommitteeInfo;

		let (current_committee_v0, next_committee_v0): (
			LegacyCommitteeInfo<
				T::ScEpochNumber,
				T::AuthorityId,
				T::AuthorityKeys,
				T::MaxValidators,
			>,
			Option<
				LegacyCommitteeInfo<
					T::ScEpochNumber,
					T::AuthorityId,
					T::AuthorityKeys,
					T::MaxValidators,
				>,
			>,
		) = Decode::decode(&mut state.as_slice())
			.expect("Previously encoded state should be decodable");

		let current_committee_v1 = crate::CurrentCommittee::<T>::get();
		let next_committee_v1 = crate::NextCommittee::<T>::get();

		ensure!(
			current_committee_v0.epoch == current_committee_v1.epoch,
			"current epoch should be preserved"
		);

		ensure!(
			current_committee_v0.committee.to_vec()
				== (current_committee_v1.committee.iter())
					.map(|member| (member.authority_id(), member.authority_keys()))
					.collect::<Vec<_>>(),
			"current committee membership should be preserved"
		);

		if next_committee_v0.is_none() && next_committee_v0.is_none() {
			return Ok(());
		}

		ensure!(next_committee_v0.is_some(), "V0 next committee should be Some if V1 is");
		ensure!(next_committee_v1.is_some(), "V1 next committee should be Some if V0 is");

		let next_committee_v0 = next_committee_v0.unwrap();
		let next_committee_v1 = next_committee_v1.unwrap();

		ensure!(
			next_committee_v0.epoch == next_committee_v1.epoch,
			"next epoch should be preserved"
		);

		ensure!(
			next_committee_v0.committee.to_vec()
				== (next_committee_v1.committee.iter())
					.map(|member| (member.authority_id(), member.authority_keys()))
					.collect::<Vec<_>>(),
			"next committee membership should be preserved"
		);

		Ok(())
	}
}
