//! Implements a re-usable migration for the authority keys type
//!
//! # Usage
//!
//! **Important**: This migration assumes that the runtime is using [pallet_session] and will
//! migrate that pallet's key storage as well.
//!
//! Authority keys migration is done by adding [AuthorityKeysMigration] to the runtime
//! migrations as part of the runtime upgrade that will change the key type.
//!
//! Preserve the old authority keys type and implement [UpgradeAuthorityKeys] trait for it.
//! For example, if a chain that originally used Aura and Grandpa keys is being upgraded to
//! also use Beefy, the definition of the legacy keys type could look like this:
//!
//! ```rust,ignore
//! impl_opaque_keys! {
//! 	#[derive(MaxEncodedLen, PartialOrd, Ord)]
//! 	pub struct LegacyAuthorityKeys {
//! 		pub aura: Aura,
//! 		pub grandpa: Grandpa,
//! 	}
//! }
//!
//! impl UpgradeAuthorityKeys<Runtime> for LegacyAuthorityKeys {
//! 	fn upgrade(
//! 		self,
//! 	) -> <Runtime as pallet_session_validator_management::Config>::AuthorityKeys {
//! 		SessionKeys {
//! 			aura: self.aura,
//! 			grandpa: self.grandpa,
//! 			beefy: ecdsa::Public::default().into(),
//! 		}
//! 	}
//! }
//! ```
//!
//! The `upgrade` implementation can arbitrarily transform the data but must not fail for any
//! of the migrated key sets.
//!
//! After implementing [UpgradeAuthorityKeys], the migration can be added to the runtime's
//! migration set:
//! ```rust,ignore
//! pub type Migrations = (
//! 	AuthorityKeysMigration<Runtime, LegacySessionKeys>,
//! 	// ...other migrations
//! );
//!
//! // ...
//!
//! pub type Executive = Executive<
//! 	Runtime,
//! 	Block,
//! 	ChainContext<Runtime>,
//! 	Runtime,
//! 	AllPalletsWithSystem,
//! 	Migrations,
//! >;
//! ```
//!
//! **Important**: Note that this migration is not versioned, and therefore *must* be removed from
//! the migration set in the code before the next runtime upgrade is performed.
extern crate alloc;
use crate::*;
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::traits::OnRuntimeUpgrade;
use sp_core::Get;
use sp_runtime::BoundedVec;
use sp_runtime::traits::{Member, OpaqueKeys};

/// Infallible cast from old to current `T::AuthorityKeys`, used for storage migration
pub trait UpgradeAuthorityKeys<T: crate::Config> {
	/// Should cast the old session keys type to the new one
	fn upgrade(self) -> T::AuthorityKeys;
}

impl<T: crate::Config> UpgradeAuthorityKeys<T> for T::AuthorityKeys {
	fn upgrade(self) -> T::AuthorityKeys {
		self
	}
}

/// Migrates existing committee members data in storage to use new types of `AuthorityId` and `AuthorityKeys`
pub struct AuthorityKeysMigration<T, OldAuthorityKeys>
where
	T: crate::Config,
	OldAuthorityKeys: UpgradeAuthorityKeys<T> + Member + Decode + Clone,
{
	_phantom: PhantomData<(T, OldAuthorityKeys)>,
}

impl<T: crate::Config, OldAuthorityKeys> AuthorityKeysMigration<T, OldAuthorityKeys>
where
	OldAuthorityKeys: UpgradeAuthorityKeys<T> + Member + Decode + Clone,
{
	/// Casts a [BoundedVec] of old committee member values to the new ones
	fn upgrade_bounded_vec(
		old: BoundedVec<CommitteeMember<T::AuthorityId, OldAuthorityKeys>, T::MaxValidators>,
	) -> BoundedVec<CommitteeMemberOf<T>, T::MaxValidators> {
		BoundedVec::truncate_from(
			old.into_iter()
				.map(|old| old.map_authority_keys(OldAuthorityKeys::upgrade))
				.collect::<Vec<_>>(),
		)
	}

	/// Casts old committee member values in a [CommitteeInfo] into new ones
	fn upgrade_committee_info(
		old: CommitteeInfo<T::AuthorityId, OldAuthorityKeys, T::MaxValidators>,
	) -> CommitteeInfoOf<T> {
		CommitteeInfo { epoch: old.epoch, committee: Self::upgrade_bounded_vec(old.committee) }
	}
}

impl<T, OldAuthorityKeys> OnRuntimeUpgrade for AuthorityKeysMigration<T, OldAuthorityKeys>
where
	T: crate::Config + pallet_session::Config<Keys = <T as crate::Config>::AuthorityKeys>,
	OldAuthorityKeys: UpgradeAuthorityKeys<T> + OpaqueKeys + Member + Decode + Clone,
{
	fn on_runtime_upgrade() -> sp_runtime::Weight {
		let mut weight = sp_runtime::Weight::zero();

		if let Some(new) = CurrentCommittee::<T>::translate::<
			CommitteeInfo<T::AuthorityId, OldAuthorityKeys, T::MaxValidators>,
			_,
		>(|old| old.map(Self::upgrade_committee_info))
		.expect("Decoding of the old value must succeed")
		{
			CurrentCommittee::<T>::put(new);
			log::info!("ℹ️ Migrated current committee storage");
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
		}

		if let Some(new) = NextCommittee::<T>::translate::<
			CommitteeInfo<T::AuthorityId, OldAuthorityKeys, T::MaxValidators>,
			_,
		>(|old| old.map(Self::upgrade_committee_info))
		.expect("Decoding of the old value must succeed")
		{
			NextCommittee::<T>::put(new);
			log::info!("ℹ️ Migrated new committee storage");
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
		}

		pallet_session::Pallet::<T>::upgrade_keys(|_id, old_keys| {
			OldAuthorityKeys::upgrade(old_keys)
		});

		weight
	}
}
