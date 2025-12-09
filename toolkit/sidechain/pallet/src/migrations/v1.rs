//! Storage migration of `pallet-sidechain` from storage version 0 to 1, removing dependence on slots
//!
//! This version change obsoletes the [SlotsPerEpoch] storage which is now deprecated
//! and will be removed in the future, and introduces a new storage [EpochDurationMillis]
//! to replace it.

/// Storage migration for chains using the slot-based legacy version of the pallet.
///
/// This migration sets the value in [EpochDurationMillis] based on the contents
/// of [SlotsPerEpoch] and slot duration.
pub type LegacyToV1Migration<T, const SLOT_DURATION_MILLIS: u64> =
	frame_support::migrations::VersionedMigration<
		0, // The migration will only execute when the on-chain storage version is 0
		1, // The on-chain storage version will be set to 1 after the migration is complete
		_impl::InnerMigrateV0ToV1<T, SLOT_DURATION_MILLIS>,
		crate::pallet::Pallet<T>,
		<T as frame_system::Config>::DbWeight,
	>;

/// Private module to void leaking
#[allow(deprecated)]
mod _impl {
	#[cfg(feature = "try-runtime")]
	extern crate alloc;

	use frame_support::traits::Get;
	use frame_support::traits::UncheckedOnRuntimeUpgrade;
	use sidechain_domain::ScEpochDuration;

	/// Helper type used internally by [LegacyToV1Migration]
	pub struct InnerMigrateV0ToV1<T: crate::Config, const SLOT_DURATION_MILLIS: u64>(
		core::marker::PhantomData<T>,
	);

	impl<T: crate::pallet::Config, const SLOT_DURATION_MILLIS: u64> UncheckedOnRuntimeUpgrade
		for InnerMigrateV0ToV1<T, SLOT_DURATION_MILLIS>
	{
		fn on_runtime_upgrade() -> sp_runtime::Weight {
			let slots_per_epoch = crate::SlotsPerEpoch::<T>::get();
			let epoch_duration_millis =
				ScEpochDuration::from_millis(slots_per_epoch as u64 * SLOT_DURATION_MILLIS);
			crate::EpochDurationMillis::<T>::put(epoch_duration_millis);

			log::info!(
				"⬆️ Migrated pallet-sidechain to version 1, with epoch duration of {epoch_duration_millis:?} ms = {slots_per_epoch} slots × {SLOT_DURATION_MILLIS} ms"
			);

			T::DbWeight::get().reads_writes(1, 1)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
			let slots_per_epoch = crate::SlotsPerEpoch::<T>::get();

			Ok(slots_per_epoch.to_be_bytes().to_vec())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
			let slots_per_epoch = u32::from_be_bytes(state.try_into().unwrap());

			let epoch_duration_millis = crate::EpochDurationMillis::<T>::get();

			frame_support::ensure!(
				slots_per_epoch as u64 * SLOT_DURATION_MILLIS == epoch_duration_millis.millis(),
				sp_runtime::TryRuntimeError::Corruption
			);

			Ok(())
		}
	}
}
