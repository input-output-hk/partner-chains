use crate::CreateChainSpecConfig;
use serde::Serialize;
use sp_core::{ecdsa, ed25519, sr25519};
use sp_runtime::traits::OpaqueKeys;

/// Trait wrapping Substrate runtime type. Should be implemented for the runtime of the node.
pub trait RuntimeTypeWrapper {
	/// Substrate runtime type.
	type Runtime;
}

/// Trait defining Partner Chain governance related types.
pub trait PartnerChainRuntime {
	/// Partner Chain authority id type
	type AuthorityId: Send + Sync + 'static + From<ecdsa::Public>;
	/// Partner Chain authority key type
	type AuthorityKeys: Send + Sync + OpaqueKeys + Serialize;
	/// Partner Chain committee member type
	type CommitteeMember: Serialize;

	/// Should construct initial [CommitteeMember] of the chain. Used for creating chain spec.
	fn initial_member(id: Self::AuthorityId, keys: Self::AuthorityKeys) -> Self::CommitteeMember;
	/// User defined function to create a chain spec given the PC configuration
	fn create_chain_spec(config: &CreateChainSpecConfig) -> serde_json::Value;
}
