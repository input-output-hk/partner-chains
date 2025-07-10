use crate::CreateChainSpecConfig;
use serde::Serialize;
use sp_core::{ecdsa, ed25519, sr25519};

/// Trait wrapping Substrate runtime type. Should be implemented for the runtime of the node.
pub trait RuntimeTypeWrapper {
	/// Substrate runtime type.
	type Runtime;
}

/// Trait defining committee pallet configuration
pub trait CommitteePaletConfig {
	type AuthorityId;
	type AuthorityKeys;
	type CommitteeMember;
}

/// Trait defining Partner Chain governance related types.
pub trait PartnerChainRuntime {
	/// Authority identifier type
	type AuthorityId: Send + Sync + 'static + From<ecdsa::Public>;
	/// Authority keys type containing session keys
	type AuthorityKeys: Send
		+ Sync
		+ 'static
		+ From<(sr25519::Public, ecdsa::Public, ed25519::Public)>
		+ Serialize;
	/// Committee member type
	type CommitteeMember: Serialize;

	/// User defined function to create a chain spec given the PC configuration
	fn create_chain_spec(config: &CreateChainSpecConfig) -> serde_json::Value;
}

pub trait PartnerChainRuntimeBindings: PartnerChainRuntime {
	fn initial_member(id: Self::AuthorityId, keys: Self::AuthorityKeys) -> Self::CommitteeMember;
}

impl<T: RuntimeTypeWrapper<Runtime = R>, R> PartnerChainRuntime for T
where
	R: CommitteePaletConfig,
	<R as CommitteePaletConfig>::AuthorityId: From<ecdsa::Public> + Send + Sync + 'static,
	<R as CommitteePaletConfig>::AuthorityKeys:
		From<(sr25519::Public, ecdsa::Public, ed25519::Public)> + Send + Sync + 'static + Serialize,
	<R as CommitteePaletConfig>::CommitteeMember: Serialize,
{
	type AuthorityId = <R as CommitteePaletConfig>::AuthorityId;
	type AuthorityKeys = <R as CommitteePaletConfig>::AuthorityKeys;
	type CommitteeMember = <R as CommitteePaletConfig>::CommitteeMember;

	fn create_chain_spec(_config: &CreateChainSpecConfig) -> serde_json::Value {
		unimplemented!("create_chain_spec must be implemented by the specific runtime")
	}
}
