//! # Primitives for Partner Chain committee selection.
//!
//! This crate implements shared types and traits used to implement Partner Chain committee rotation.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

extern crate alloc;

use alloc::vec::Vec;
#[cfg(feature = "std")]
use core::str::FromStr;
use parity_scale_codec::DecodeWithMemTracking;
use scale_info::TypeInfo;
use sidechain_domain::{
	CandidateRegistrations, DParameter, EpochNonce, MainchainAddress, PermissionedCandidateData,
	PolicyId, StakePoolPublicKey, byte_string::SizedByteString,
};
use sp_core::{Decode, Encode, MaxEncodedLen};
use sp_inherents::{InherentIdentifier, IsFatalError};

/// Inherent identifier used by the Committee Selection pallet
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"/ariadne";

#[derive(Encode, sp_runtime::RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
/// Error type used for failing calls of the Committee Selection inherent.
pub enum InherentError {
	#[deprecated(
		since = "1.5.0",
		note = "Use InvalidValidatorsMatchingHash or InvalidValidatorsHashMismatch"
	)]
	#[cfg_attr(
		feature = "std",
		error("The validators in the block do not match the calculated validators")
	)]
	/// The validators in the block do not match the calculated validators
	InvalidValidators,
	#[cfg_attr(
		feature = "std",
		error("Candidates inherent required: committee needs to be stored one epoch in advance")
	)]
	/// Candidates inherent required: committee needs to be stored one epoch in advance
	CommitteeNeedsToBeStoredOneEpochInAdvance,
	#[cfg_attr(
		feature = "std",
		error("The validators in the block do not match the calculated validators. Input data hash ({}) is valid.", .0.to_hex_string())
	)]
	/// The validators in the block do not match the calculated validators, but the input data hash is valid.
	InvalidValidatorsMatchingHash(SizedByteString<32>),
	#[cfg_attr(
		feature = "std",
		error("The validators and input data hash in the block do not match the calculated values. Expected hash: {}, got: {}",
			.0.to_hex_string(),
			.1.to_hex_string())
	)]
	/// The validators and input data hash in the block do not match the calculated values.
	InvalidValidatorsHashMismatch(SizedByteString<32>, SizedByteString<32>),
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
	}
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	MaxEncodedLen,
	Debug,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
)]
/// Type representing committee members, either permissioned or registered
pub enum CommitteeMember<AuthorityId, AuthorityKeys> {
	/// A permissioned candidate
	Permissioned {
		/// Authority id of the candidate
		id: AuthorityId,
		/// Authority keys of the candidate
		keys: AuthorityKeys,
	},
	/// A registered candidate
	Registered {
		/// Authority id of the candidate
		id: AuthorityId,
		/// Authority keys of the candidate
		keys: AuthorityKeys,
		/// Stake pool pub key of the candidate
		stake_pool_pub_key: StakePoolPublicKey,
	},
}

impl<AuthorityId, AuthorityKeys> From<(AuthorityId, AuthorityKeys)>
	for CommitteeMember<AuthorityId, AuthorityKeys>
{
	fn from((id, keys): (AuthorityId, AuthorityKeys)) -> Self {
		Self::Permissioned { id, keys }
	}
}

impl<AuthorityId, AuthorityKeys> CommitteeMember<AuthorityId, AuthorityKeys> {
	/// Constructs new permissioned candidate
	pub fn permissioned(id: AuthorityId, keys: AuthorityKeys) -> Self {
		Self::Permissioned { id, keys }
	}

	/// Returns the authority ID of the committee member
	pub fn authority_id(&self) -> AuthorityId
	where
		AuthorityId: Clone,
	{
		match self {
			Self::Permissioned { id, .. } => id.clone(),
			Self::Registered { id, .. } => id.clone(),
		}
	}

	/// Returns the authority keys of the committee member
	pub fn authority_keys(&self) -> AuthorityKeys
	where
		AuthorityKeys: Clone,
	{
		match self {
			Self::Permissioned { keys, .. } => keys.clone(),
			Self::Registered { keys, .. } => keys.clone(),
		}
	}
}

#[cfg(feature = "std")]
impl From<InherentError> for sp_inherents::Error {
	fn from(value: InherentError) -> Self {
		sp_inherents::Error::Application(Box::from(value))
	}
}

#[derive(Default, Debug, Clone, PartialEq, Eq, TypeInfo, Encode, Decode, MaxEncodedLen)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Collection of all mainchain script info needed for committee selection
pub struct MainChainScripts {
	/// [MainchainAddress] where registration UTXOs are located
	pub committee_candidate_address: MainchainAddress,
	/// [PolicyId] of D-parameter script
	pub d_parameter_policy_id: PolicyId,
	/// [PolicyId] of Permissioned Candidates script
	pub permissioned_candidates_policy_id: PolicyId,
}

#[cfg(feature = "std")]
impl MainChainScripts {
	/// Reads [MainChainScripts] from env vars:
	/// - COMMITTEE_CANDIDATE_ADDRESS
	/// - D_PARAMETER_POLICY_ID
	/// - PERMISSIONED_CANDIDATES_POLICY_ID
	pub fn read_from_env() -> Result<MainChainScripts, envy::Error> {
		#[derive(serde::Serialize, serde::Deserialize)]
		pub struct MainChainScriptsEnvConfig {
			pub committee_candidate_address: String,
			pub d_parameter_policy_id: PolicyId,
			pub permissioned_candidates_policy_id: PolicyId,
		}

		let MainChainScriptsEnvConfig {
			committee_candidate_address,
			d_parameter_policy_id,
			permissioned_candidates_policy_id,
		} = envy::from_env::<MainChainScriptsEnvConfig>()?;

		let committee_candidate_address = FromStr::from_str(&committee_candidate_address)
			.map_err(|err| envy::Error::Custom(format!("Incorrect main chain address: {}", err)))?;

		Ok(Self {
			committee_candidate_address,
			d_parameter_policy_id,
			permissioned_candidates_policy_id,
		})
	}
}

/// The part of data for selection of authorities that comes from the main chain.
/// It is unfiltered, so the selection algorithm should filter out invalid candidates.
#[derive(Clone, Debug, Encode, Decode, DecodeWithMemTracking, TypeInfo, PartialEq, Eq)]
pub struct AuthoritySelectionInputs {
	/// D-parameter for Ariadne committee selection. See [DParameter] for details.
	pub d_parameter: DParameter,
	/// List of permissioned candidates for committee selection.
	pub permissioned_candidates: Vec<PermissionedCandidateData>,
	/// List of registered candidates for committee selection
	pub registered_candidates: Vec<CandidateRegistrations>,
	/// Nonce for queried epoch.
	pub epoch_nonce: EpochNonce,
}

sp_api::decl_runtime_apis! {
	#[api_version(3)]
	/// Runtime API declaration for Session Validator Management
	pub trait SessionValidatorManagementApi<
		AuthorityId,
		AuthorityKeys,
		ScEpochNumber: parity_scale_codec::Encode + parity_scale_codec::Decode
	> where
		AuthorityId: Encode + Decode,
		AuthorityKeys: Encode + Decode,
	{
		/// Returns main chain scripts
		fn get_main_chain_scripts() -> MainChainScripts;
		/// Returns next unset [sidechain_domain::ScEpochNumber]
		fn get_next_unset_epoch_number() -> ScEpochNumber;

		#[changed_in(2)]
		/// Returns current committee
		fn get_current_committee() -> (ScEpochNumber, sp_std::vec::Vec<CommitteeMember<AuthorityId, AuthorityKeys>>);
		/// Returns current committee
		fn get_current_committee() -> (ScEpochNumber, sp_std::vec::Vec<CommitteeMember<AuthorityId, AuthorityKeys>>);

		#[changed_in(2)]
		/// Returns next committee
		fn get_next_committee() -> Option<(ScEpochNumber, sp_std::vec::Vec<CommitteeMember<AuthorityId, AuthorityKeys>>)>;
		/// Returns next committee
		fn get_next_committee() -> Option<(ScEpochNumber, sp_std::vec::Vec<CommitteeMember<AuthorityId, AuthorityKeys>>)>;

		#[changed_in(2)]
		/// Calculates committee
		fn calculate_committee(
			authority_selection_inputs: AuthoritySelectionInputs,
			sidechain_epoch: ScEpochNumber
		) -> Option<sp_std::vec::Vec<(AuthorityId, AuthorityKeys)>>;

		/// Calculates committee
		fn calculate_committee(
			authority_selection_inputs: AuthoritySelectionInputs,
			sidechain_epoch: ScEpochNumber
		) -> Option<sp_std::vec::Vec<CommitteeMember<AuthorityId, AuthorityKeys>>>;
	}
}
