#[cfg(feature = "std")]
/// Defines parts of inherent data that should be included in header digest
pub trait InherentDigest {
	/// Rust type of the inherent digest value
	type Value: Send + Sync + 'static;

	/// Construct digest items from block's inherent data
	fn from_inherent_data(
		inherent_data: &sp_inherents::InherentData,
	) -> Result<Vec<sp_runtime::DigestItem>, Box<dyn std::error::Error + Send + Sync>>;

	/// Retrieve value from digests
	fn value_from_digest(
		digests: &[sp_runtime::DigestItem],
	) -> Result<Self::Value, Box<dyn std::error::Error + Send + Sync>>;
}

#[cfg(feature = "std")]
impl InherentDigest for () {
	type Value = ();

	fn from_inherent_data(
		_inherent_data: &sp_inherents::InherentData,
	) -> Result<Vec<sp_runtime::DigestItem>, Box<dyn std::error::Error + Send + Sync>> {
		Ok(vec![])
	}

	fn value_from_digest(
		_digests: &[sp_runtime::DigestItem],
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		Ok(())
	}
}
