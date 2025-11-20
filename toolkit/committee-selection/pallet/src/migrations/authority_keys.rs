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
//! ```rust
//! use pallet_session_validator_management::migrations::authority_keys::UpgradeAuthorityKeys;
//! use sp_core::*;
//! use sp_runtime::impl_opaque_keys;
//!
//! # use sp_runtime::BoundToRuntimeAppPublic;
//!
//! # pub struct Aura;
//! # impl BoundToRuntimeAppPublic for Aura { type Public = sp_runtime::app_crypto::sr25519::AppPublic; }
//! # pub struct Grandpa;
//! # impl BoundToRuntimeAppPublic for Grandpa { type Public = sp_runtime::app_crypto::ed25519::AppPublic; }
//! # pub struct Beefy;
//! # impl BoundToRuntimeAppPublic for Beefy { type Public = sp_runtime::app_crypto::ecdsa::AppPublic; }
//!
//! impl_opaque_keys! {
//! 	pub struct AuthorityKeys {
//! 		pub aura: Aura,
//! 		pub grandpa: Grandpa,
//! 		pub beefy: Beefy,
//! 	}
//! }
//!
//! impl_opaque_keys! {
//! 	pub struct LegacyAuthorityKeys {
//! 		pub aura: Aura,
//! 		pub grandpa: Grandpa,
//! 	}
//! }
//!
//! impl UpgradeAuthorityKeys<AuthorityKeys> for LegacyAuthorityKeys {
//! 	fn upgrade(self) -> AuthorityKeys {
//! 		AuthorityKeys {
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
//! 	AuthorityKeysMigration<Runtime, LegacySessionKeys, 0, 1>,
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
//! Note that [AuthorityKeysMigration] is parametrized by the session keys versions from which
//! and to which it migrates, to guarantee idempotency. Current session keys version can be
//! obtained by reading the [AuthorityKeysVersion] storage and by default starts as 0.

extern crate alloc;

use crate::*;
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::traits::OnRuntimeUpgrade;
use sp_core::Get;
use sp_runtime::BoundedVec;
use sp_runtime::traits::{Member, OpaqueKeys};

/// Infallible cast from old to current `T::AuthorityKeys`, used for storage migration
pub trait UpgradeAuthorityKeys<NewAuthorityKeys> {
	/// Should cast the old session keys type to the new one
	fn upgrade(self) -> NewAuthorityKeys;
}

impl<T> UpgradeAuthorityKeys<T> for T {
	fn upgrade(self) -> T {
		self
	}
}

/// Migrates existing committee members data in storage to use new type `AuthorityKeys`
///
/// This migration is versioned and will only applied when on-chain session keys version
/// as read from [AuthorityKeysVersion] storage is equal to `FROM_VERSION` and will
/// set the version to `TO_VERSION`.
///
/// **Important**: This migration assumes that the runtime is using [pallet_session] and will
/// migrate that pallet's key storage as well.
pub struct AuthorityKeysMigration<
	T,
	OldAuthorityKeys,
	const FROM_VERSION: u32,
	const TO_VERSION: u32,
> where
	T: crate::Config,
	OldAuthorityKeys: UpgradeAuthorityKeys<T::AuthorityKeys> + Member + Decode + Clone,
{
	_phantom: PhantomData<(T, OldAuthorityKeys)>,
}

impl<T: crate::Config, OldAuthorityKeys, const FROM_VERSION: u32, const TO_VERSION: u32>
	AuthorityKeysMigration<T, OldAuthorityKeys, FROM_VERSION, TO_VERSION>
where
	OldAuthorityKeys: UpgradeAuthorityKeys<T::AuthorityKeys> + Member + Decode + Clone,
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

impl<T, OldAuthorityKeys, const FROM_VERSION: u32, const TO_VERSION: u32> OnRuntimeUpgrade
	for AuthorityKeysMigration<T, OldAuthorityKeys, FROM_VERSION, TO_VERSION>
where
	T: crate::Config + pallet_session::Config<Keys = <T as crate::Config>::AuthorityKeys>,
	OldAuthorityKeys: UpgradeAuthorityKeys<T::AuthorityKeys> + OpaqueKeys + Member + Decode + Clone,
{
	fn on_runtime_upgrade() -> sp_runtime::Weight {
		let current_version = crate::AuthorityKeysVersion::<T>::get();

		let mut weight = T::DbWeight::get().reads_writes(1, 0);

		if TO_VERSION <= current_version {
			log::warn!(
				"🚚 AuthorityKeysMigration {FROM_VERSION}->{TO_VERSION} can be removed; storage is already at version {current_version}."
			);
			return weight;
		}
		if current_version != FROM_VERSION {
			log::warn!(
				"🚚 AuthorityKeysMigration {FROM_VERSION}->{TO_VERSION} can not be applied to storage at version {current_version}."
			);
			return weight;
		}

		if let Some(new) = CurrentCommittee::<T>::translate::<
			CommitteeInfo<T::AuthorityId, OldAuthorityKeys, T::MaxValidators>,
			_,
		>(|old| old.map(Self::upgrade_committee_info))
		.expect("Decoding of the old value must succeed")
		{
			CurrentCommittee::<T>::put(new);
			log::info!("🚚️ Migrated current committee storage to version {TO_VERSION}");
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
		}

		if let Some(new) = NextCommittee::<T>::translate::<
			CommitteeInfo<T::AuthorityId, OldAuthorityKeys, T::MaxValidators>,
			_,
		>(|old| old.map(Self::upgrade_committee_info))
		.expect("Decoding of the old value must succeed")
		{
			NextCommittee::<T>::put(new);
			log::info!("🚚️ Migrated new committee storage to version {TO_VERSION}");
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
		}

		pallet_session::Pallet::<T>::upgrade_keys(|_id, old_keys| {
			OldAuthorityKeys::upgrade(old_keys)
		});
		weight.saturating_add(T::DbWeight::get().reads_writes(2, 2));
		log::info!("🚚️ Migrated keys in pallet_session to version {TO_VERSION}");

		crate::AuthorityKeysVersion::<T>::set(TO_VERSION);
		weight.saturating_add(T::DbWeight::get().reads_writes(0, 1));

		weight
	}
}
