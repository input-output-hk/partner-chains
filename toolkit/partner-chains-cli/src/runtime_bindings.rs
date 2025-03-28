use pallet_session_validator_management::Config as CommitteePaletConfig;
use serde::Serialize;
use sp_core::{ecdsa, ed25519, sr25519};
use sp_session_validator_management::CommitteeMember as CommitteeMemberT;

pub trait RuntimeTypeWrapper {
	type Runtime;
}

pub trait PartnerChainRuntime {
	type AuthorityId: Send + Sync + 'static + From<ecdsa::Public>;
	type AuthorityKeys: Send + Sync + 'static + From<(sr25519::Public, ed25519::Public)> + Serialize;
	type CommitteeMember: Serialize;
}

pub trait PartnerChainRuntimeBindings: PartnerChainRuntime {
	fn initial_member(id: Self::AuthorityId, keys: Self::AuthorityKeys) -> Self::CommitteeMember;
}

impl<T: RuntimeTypeWrapper<Runtime = R>, R> PartnerChainRuntime for T
where
	R: CommitteePaletConfig,
	<R as CommitteePaletConfig>::AuthorityId: From<ecdsa::Public>,
	<R as CommitteePaletConfig>::AuthorityKeys: From<(sr25519::Public, ed25519::Public)>,
	<R as CommitteePaletConfig>::CommitteeMember: Serialize,
{
	type AuthorityId = <R as CommitteePaletConfig>::AuthorityId;
	type AuthorityKeys = <R as CommitteePaletConfig>::AuthorityKeys;
	type CommitteeMember = <R as CommitteePaletConfig>::CommitteeMember;
}
