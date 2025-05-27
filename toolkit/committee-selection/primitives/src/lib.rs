//! Primitives for `committee-selection`.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

#[cfg(feature = "std")]
use core::str::FromStr;

use scale_info::TypeInfo;
use sidechain_domain::{MainchainAddress, PolicyId, byte_string::SizedByteString};
use sp_core::{Decode, Encode, MaxEncodedLen};
use sp_inherents::{InherentIdentifier, IsFatalError};

/// Inherent identifier used by the Committee Selection pallet
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"/ariadne";

#[derive(Encode, sp_runtime::RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
/// Error type used for failing calls of the Committee Selection inherent.
pub enum InherentError {
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
	#[deprecated(since = "1.7.0", note = "Use InvalidValidators")]
	#[cfg_attr(
		feature = "std",
		error("The validators in the block do not match the calculated validators. Input data hash ({}) is valid.", .0.to_hex_string())
	)]
	/// The validators in the block do not match the calculated validators, but the input data hash is valid.
	InvalidValidatorsMatchingHash(SizedByteString<32>),
	#[deprecated(since = "1.7.0", note = "Use InvalidValidators")]
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

/// Signifies that a type represents a committee member
pub trait CommitteeMember {
	/// Type representing authority id
	type AuthorityId;
	/// Type representing authority keys
	type AuthorityKeys;
	/// Returns authority id
	fn authority_id(&self) -> Self::AuthorityId;
	/// Returns authority keys
	fn authority_keys(&self) -> Self::AuthorityKeys;
}
impl<AuthorityId: Clone, AuthorityKeys: Clone> CommitteeMember for (AuthorityId, AuthorityKeys) {
	type AuthorityId = AuthorityId;
	type AuthorityKeys = AuthorityKeys;
	fn authority_id(&self) -> AuthorityId {
		self.0.clone()
	}
	fn authority_keys(&self) -> AuthorityKeys {
		self.1.clone()
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

sp_api::decl_runtime_apis! {
	#[api_version(2)]
	/// Runtime API declaration for Session Validator Management
	pub trait SessionValidatorManagementApi<
		CommitteeMember: parity_scale_codec::Decode + parity_scale_codec::Encode + crate::CommitteeMember,
		AuthoritySelectionInputs: parity_scale_codec::Encode,
		ScEpochNumber: parity_scale_codec::Encode + parity_scale_codec::Decode
	> where
	CommitteeMember::AuthorityId: Encode + Decode,
	CommitteeMember::AuthorityKeys: Encode + Decode,
	{
		/// Returns main chain scripts
		fn get_main_chain_scripts() -> MainChainScripts;
		/// Returns next unset [ScEpochNumber]
		fn get_next_unset_epoch_number() -> ScEpochNumber;

		#[changed_in(2)]
		/// Returns current committee
		fn get_current_committee() -> (ScEpochNumber, sp_std::vec::Vec<CommitteeMember::AuthorityId>);
		/// Returns current committee
		fn get_current_committee() -> (ScEpochNumber, sp_std::vec::Vec<CommitteeMember>);

		#[changed_in(2)]
		/// Returns next committee
		fn get_next_committee() -> Option<(ScEpochNumber, sp_std::vec::Vec<CommitteeMember::AuthorityId>)>;
		/// Returns next committee
		fn get_next_committee() -> Option<(ScEpochNumber, sp_std::vec::Vec<CommitteeMember>)>;

		#[changed_in(2)]
		/// Calculates committee
		fn calculate_committee(
			authority_selection_inputs: AuthoritySelectionInputs,
			sidechain_epoch: ScEpochNumber
		) -> Option<sp_std::vec::Vec<(CommitteeMember::AuthorityId, CommitteeMember::AuthorityKeys)>>;

		/// Calculates committee
		fn calculate_committee(
			authority_selection_inputs: AuthoritySelectionInputs,
			sidechain_epoch: ScEpochNumber
		) -> Option<sp_std::vec::Vec<CommitteeMember>>;
	}
}
