#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use sp_inherents::IsFatalError;

pub use crate::inherent_data::{
	AddressAssociationInherentData, AddressAssociationsInherentDataProvider, INHERENT_IDENTIFIER,
};
pub use sp_inherents::InherentData;

#[cfg(feature = "cardano")]
pub mod cardano;
pub mod inherent_data;

#[derive(Encode, PartialEq)]
#[cfg_attr(not(feature = "std"), derive(Debug))]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error, sp_runtime::RuntimeDebug))]
pub enum InherentError {
	#[cfg_attr(feature = "std", error("Address association inherent missing when expected."))]
	InherentRequired,
	#[cfg_attr(
		feature = "std",
		error("Address associations in the inherent do not match inherent data.")
	)]
	IncorrectAssociations,
	#[cfg_attr(
		feature = "std",
		error("New sync state in the inherent does not match inherent data.")
	)]
	IncorrectNewSyncState,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

sp_api::decl_runtime_apis! {
	/// User-facing runtime API defined by the address association pallet
	pub trait AddressAssociationsApi {
		/// Get the current version of the pallet.
		///
		/// This version is distinct from the API version.
		fn get_version() -> u32;
	}

	/// Runtime API exposed by the address association pallet to serve operational data to the observability layer
	pub trait AddressAssociationsObservabilityApi<
		ObservabilityConfigurationType: parity_scale_codec::Decode,
		SyncStateType: parity_scale_codec::Decode
	> {
		/// Get the current observability configuration stored in the pallet.
		fn get_observability_configuration() -> Option<ObservabilityConfigurationType>;

		/// Get the current sync state stored in the pallet.
		fn get_current_sync_state() -> Option<SyncStateType>;
	}
}
