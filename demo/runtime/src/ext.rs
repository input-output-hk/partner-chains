use core::marker::PhantomData;

use frame_support::traits::EstimateNextSessionRotation;
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_partner_chains_session::ShouldEndSession;
use sp_runtime::{
	traits::{One, Zero},
	Permill,
};
use sp_weights::Weight;

pub struct PartnerChainsSessionFrameSupport<T>(PhantomData<T>);

impl<T> EstimateNextSessionRotation<BlockNumberFor<T>> for PartnerChainsSessionFrameSupport<T>
where
	T: pallet_partner_chains_session::Config
		+ pallet_session_validator_management::Config
		+ pallet_sidechain::Config,
{
	fn average_session_length() -> BlockNumberFor<T> {
		let spe = pallet_sidechain::Pallet::<T>::slots_per_epoch();
		let bn = BlockNumberFor::<T>::from(spe.0);
		let msg = alloc::format!("average session length: {bn:?}");
		sp_io::logging::log(sp_core::LogLevel::Error, "stdout", msg.as_bytes());
		bn
	}

	//TODO: weights
	fn estimate_current_session_progress(now: BlockNumberFor<T>) -> (Option<Permill>, Weight) {
		if T::ShouldEndSession::should_end_session(now) {
			// Should surely end session
			let msg = alloc::format!("should end session is true for {now:?}");
			sp_io::logging::log(sp_core::LogLevel::Error, "stdout", msg.as_bytes());
			(Some(Permill::one()), Zero::zero())
		} else {
			let slots_per_epoch = pallet_sidechain::Pallet::<T>::slots_per_epoch();
			let current_slot = T::current_slot_number();
			let slot_in_epoch = slots_per_epoch.slot_number_in_epoch(current_slot.0.into());
			let progress = Permill::from_rational(slot_in_epoch, slots_per_epoch.0);
			let msg = alloc::format!("estimate_current_session_progress: {progress:?}");
			sp_io::logging::log(sp_core::LogLevel::Error, "stdout", msg.as_bytes());
			(Some(progress), Zero::zero())
		}
	}

	fn estimate_next_session_rotation(
		block_now: BlockNumberFor<T>,
	) -> (Option<BlockNumberFor<T>>, Weight) {
		if T::ShouldEndSession::should_end_session(block_now) {
			// Should surely end session
			(Some(block_now + One::one()), Zero::zero())
		} else {
			let slots_per_epoch = pallet_sidechain::Pallet::<T>::slots_per_epoch();
			let current_slot = T::current_slot_number();
			let slot_in_epoch = slots_per_epoch.slot_number_in_epoch(current_slot.0.into());
			let slots_left = slots_per_epoch.0 - slot_in_epoch;
			(Some(block_now + slots_left.into()), Zero::zero())
		}
	}
}
