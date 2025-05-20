//! Primitives for block producer fees feature
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]

use parity_scale_codec::Decode;

/// Margin Fee precision is 0.01 of a percent, so 1/10000 is used as a unit.
pub type PerTenThousands = u16;

sp_api::decl_runtime_apis! {
	/// Runtime API for block producer fees. Required for convenient access to the data by RPC.
	pub trait BlockProducerFeesApi<AccountId: Decode>
	{
		/// Retrieves the latests fees of all accounts that have set them.
		fn get_all_fees() -> sp_std::vec::Vec<(AccountId, PerTenThousands)>;
	}
}
