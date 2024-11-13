#![cfg_attr(not(feature = "std"), no_std)]

use sp_staking::SessionIndex;
use sp_std::prelude::*;

/// Use this when you need to have a pallet-session Config implemented in your runtime.
pub struct PalletSessionStubImpls;

impl<T> pallet_session::ShouldEndSession<T> for PalletSessionStubImpls {
	fn should_end_session(_: T) -> bool {
		false
	}
}

impl<T> pallet_session::SessionManager<T> for PalletSessionStubImpls {
	fn new_session(_: SessionIndex) -> Option<Vec<T>> {
		None
	}

	fn end_session(_: SessionIndex) {}

	fn start_session(_: SessionIndex) {}
}

impl<T> sp_runtime::traits::Convert<T, Option<T>> for PalletSessionStubImpls {
	fn convert(t: T) -> Option<T> {
		Some(t)
	}
}

/// Macro to implement `pallet_session::Config`, using existing `pallet_partner_chains_session::Config
/// Example usage: impl_pallet_session_config!(Runtime);
#[macro_export]
macro_rules! impl_pallet_session_config {
	($type:ty) => {
		impl pallet_session::Config for $type
		where
			$type: pallet_partner_chains_session::Config,
		{
			type RuntimeEvent = <$type as pallet_partner_chains_session::Config>::RuntimeEvent;
			type ValidatorId = <$type as pallet_partner_chains_session::Config>::ValidatorId;
			type ValidatorIdOf = pallet_session_runtime_stub::PalletSessionStubImpls;
			type ShouldEndSession = pallet_session_runtime_stub::PalletSessionStubImpls;
			type NextSessionRotation = ();
			type SessionManager = pallet_session_runtime_stub::PalletSessionStubImpls;
			type SessionHandler = <$type as pallet_partner_chains_session::Config>::SessionHandler;
			type Keys = <$type as pallet_partner_chains_session::Config>::Keys;
			type WeightInfo = ();
		}
	};
}
