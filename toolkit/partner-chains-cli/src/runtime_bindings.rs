use crate::CreateChainSpecConfig;

/// Trait wrapping Substrate runtime type. Should be implemented for the runtime of the node.
pub trait RuntimeTypeWrapper {
	/// Substrate runtime type.
	type Runtime;
}

/// Trait defining Partner Chain governance related types.
pub trait PartnerChainRuntime {
	/// User defined function to create a chain spec given the PC configuration
	fn create_chain_spec(config: &CreateChainSpecConfig) -> serde_json::Value;
}
