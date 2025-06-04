use serde::Serialize;
use sp_core::{ecdsa, ed25519, sr25519};

pub trait RuntimeTypeWrapper {
	type Runtime;
}

pub trait PartnerChainRuntime {
	type AuthorityId: Send + Sync + 'static + From<ecdsa::Public>;
	type AuthorityKeys: Send + Sync + 'static + From<(sr25519::Public, ed25519::Public)> + Serialize;
	type CommitteeMember: Serialize;

	/// Should construct initial [CommitteeMember] of the chain. Used for creating chain spec.
	fn initial_member(id: Self::AuthorityId, keys: Self::AuthorityKeys) -> Self::CommitteeMember;
}
