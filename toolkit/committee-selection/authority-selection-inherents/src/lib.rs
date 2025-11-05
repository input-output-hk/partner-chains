//! # Partner Chain Committee Selection
//!
//! Inherent data provider and selection logic for Partner Chain committee selection.
//!
//! ## Overview
//!
//! This crate provides an IDP and all types necessary for a Partner Chain to select
//! block producer committees using data sourced from Cardano smart contracts.
//!
//! ## Usage
//!
//! ### Prerequisites
//!
//! This crate is intended to work with `pallet_session_validator_management`. See
//! the pallet's documentation for instructions how to add it to you runtime.
//!
//! Additionally [AriadneInherentDataProvider] needs access to a data source
//! implementing [AuthoritySelectionDataSource]. A Db-Sync-based implementation is
//! provided by the `partner_chains_db_sync_data_sources` crate.
//!
//! ### Adding to the node
//!
//! #### Implementing runtime API
//!
//! Implement the [SessionValidatorManagementApi] for your runtime. Each API method has
//! a corresponding method in the pallet that should be used for that purpose. Refer to
//! the demo runtime for an example.
//!
//! #### Add the inherent data provider
//!
//! Wire the [AriadneInherentDataProvider] into your inherent data provider stack. The same
//! constructor [AriadneInherentDataProvider::new] should be used for both proposing and
//! validating blocks. Refer to the demo node implementation for an example of how to wire
//! it correctly into a node.
//!
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sidechain_domain::{CandidateKeys, StakePoolPublicKey};
use sp_core::{ConstU32, Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use sp_runtime::traits::OpaqueKeys;

mod ariadne_inherent_data_provider;
mod authority_selection_inputs;
mod filter_invalid_candidates;
mod select_authorities;

pub use {
	ariadne_inherent_data_provider::AriadneInherentDataProvider,
	authority_selection_inputs::{AriadneParameters, AuthoritySelectionInputs},
	filter_invalid_candidates::{
		PermissionedCandidateDataError, RegisterValidatorSignedMessage, RegistrationDataError,
		StakeError, filter_trustless_candidates_registrations,
		runtime_decl_for_candidate_validation_api, validate_permissioned_candidate_data,
		validate_registration_data, validate_stake,
	},
	select_authorities::select_authorities,
};
#[cfg(feature = "std")]
pub use {
	authority_selection_inputs::AuthoritySelectionDataSource,
	filter_invalid_candidates::CandidateValidationApi,
};

#[cfg(test)]
mod runtime_api_mock;
#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

/// Trait to try extract implementing type from [CandidateKeys].
pub trait MaybeFromCandidateKeys: OpaqueKeys + Decode + Sized {
	/// Depends on `Decode` that is derived by `impl_opaque_keys!`
	fn maybe_from(keys: &CandidateKeys) -> Option<Self> {
		let required_keys = Self::key_ids();

		let mut encoded_keys = sp_runtime::BoundedVec::<u8, ConstU32<1024>>::new();
		for key_id in required_keys {
			let key = keys.0.iter().find(|key| key.id == key_id.0)?;
			encoded_keys.try_append(&mut key.bytes.clone()).ok()?;
		}
		Self::decode(&mut &encoded_keys[..]).ok()
	}
}
