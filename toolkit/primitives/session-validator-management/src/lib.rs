#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use core::str::FromStr;

use scale_info::TypeInfo;
use sidechain_domain::{byte_string::SizedByteString, MainchainAddress, PolicyId};
use sp_core::{Decode, Encode, MaxEncodedLen};
use sp_inherents::{InherentIdentifier, IsFatalError};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"/ariadne";

#[derive(Encode, sp_runtime::RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	#[deprecated(
		since = "1.5.0",
		note = "Use InvalidValidatorsMatchingHash or InvalidValidatorsHashMismatch"
	)]
	#[cfg_attr(
		feature = "std",
		error("The validators in the block do not match the calculated validators")
	)]
	InvalidValidators,
	#[cfg_attr(
		feature = "std",
		error("Candidates inherent required: committee needs to be stored one epoch in advance")
	)]
	CommitteeNeedsToBeStoredOneEpochInAdvance,
	#[cfg_attr(
		feature = "std",
		error("The validators in the block do not match the calculated validators. Input data hash ({}) is valid.", .0.to_hex_string())
	)]
	InvalidValidatorsMatchingHash(SizedByteString<32>),
	#[cfg_attr(
		feature = "std",
		error("The validators and input data hash in the block do not match the calculated values. Expected hash: {}, got: {}",
			.0.to_hex_string(),
			.1.to_hex_string())
	)]
	InvalidValidatorsHashMismatch(SizedByteString<32>, SizedByteString<32>),
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		true
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
pub struct MainChainScripts {
	pub committee_candidate_address: MainchainAddress,
	pub d_parameter_policy_id: PolicyId,
	pub permissioned_candidates_policy_id: PolicyId,
}

#[cfg(feature = "std")]
impl MainChainScripts {
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
	pub trait SessionValidatorManagementApi<
		SessionKeys: parity_scale_codec::Decode,
		CrossChainPublic: parity_scale_codec::Decode + parity_scale_codec::Encode,
		AuthoritySelectionInputs: parity_scale_codec::Encode,
		ScEpochNumber: parity_scale_codec::Encode + parity_scale_codec::Decode
	> {
		fn get_main_chain_scripts() -> MainChainScripts;
		fn get_current_committee() -> (ScEpochNumber, sp_std::vec::Vec<CrossChainPublic>);
		fn get_next_committee() -> Option<(ScEpochNumber, sp_std::vec::Vec<CrossChainPublic>)>;
		fn get_next_unset_epoch_number() -> ScEpochNumber;
		fn calculate_committee(
			authority_selection_inputs: AuthoritySelectionInputs,
			sidechain_epoch: ScEpochNumber
		) -> Option<sp_std::vec::Vec<(CrossChainPublic, SessionKeys)>>;
	}
}
