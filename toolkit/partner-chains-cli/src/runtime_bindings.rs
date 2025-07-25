use crate::{CreateChainSpecConfig, keystore::KeyDefinition};
use authority_selection_inherents::MaybeFromCandidateKeys;

/// Trait defining Partner Chain governance related types.
pub trait PartnerChainRuntime {
	/// User keys type
	type Keys: MaybeFromCandidateKeys;
	/// User defined function to create a chain spec given the PC configuration
	fn create_chain_spec(config: &CreateChainSpecConfig<Self::Keys>) -> serde_json::Value;
	/// Names and schemes of keys used by the runtime.
	fn key_definitions() -> Vec<KeyDefinition<'static>>;
}
