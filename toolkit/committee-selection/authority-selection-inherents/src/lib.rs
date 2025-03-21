#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sidechain_domain::StakePoolPublicKey;
use sp_core::{Decode, Encode, MaxEncodedLen};
use sp_session_validator_management::CommitteeMember as CommitteeMemberT;

pub mod ariadne_inherent_data_provider;
pub mod authority_selection_inputs;
pub mod filter_invalid_candidates;
pub mod select_authorities;

#[cfg(test)]
mod runtime_api_mock;
#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "mock"))]
pub mod mock;

#[derive(
	Serialize, Deserialize, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq,
)]
pub enum CommitteeMember<AuthorityId, AuthorityKeys> {
	Permissioned { id: AuthorityId, keys: AuthorityKeys },
	Registered { id: AuthorityId, keys: AuthorityKeys, stake_pool_pub_key: StakePoolPublicKey },
}

impl<AuthorityId, AuthorityKeys> From<(AuthorityId, AuthorityKeys)>
	for CommitteeMember<AuthorityId, AuthorityKeys>
{
	fn from((id, keys): (AuthorityId, AuthorityKeys)) -> Self {
		Self::Permissioned { id, keys }
	}
}

impl<AuthorityId, AuthorityKeys> CommitteeMember<AuthorityId, AuthorityKeys> {
	pub fn permissioned(id: AuthorityId, keys: AuthorityKeys) -> Self {
		Self::Permissioned { id, keys }
	}
}

impl<AuthorityId: Clone, AuthorityKeys: Clone> CommitteeMemberT
	for CommitteeMember<AuthorityId, AuthorityKeys>
{
	type AuthorityId = AuthorityId;
	type AuthorityKeys = AuthorityKeys;

	fn authority_id(&self) -> AuthorityId {
		match self {
			Self::Permissioned { id, .. } => id.clone(),
			Self::Registered { id, .. } => id.clone(),
		}
	}

	fn authority_keys(&self) -> AuthorityKeys {
		match self {
			Self::Permissioned { keys, .. } => keys.clone(),
			Self::Registered { keys, .. } => keys.clone(),
		}
	}
}
