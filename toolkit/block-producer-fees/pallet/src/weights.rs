//! Weights for runtime calls, output of benchmarking
use frame_support::weights::Weight;

/// Weight functions needed for the pallet.
pub trait WeightInfo {
	/// Weight of set_fee
	fn set_fee() -> Weight;
}

impl WeightInfo for () {
	fn set_fee() -> Weight {
		Weight::zero()
	}
}
