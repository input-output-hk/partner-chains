//! Benchmarking setup for pallet-governed-map

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

/// Trait for injecting chain-specific test values for benchmarking.
pub trait BenchmarkHelper<T: crate::Config> {
	/// Returns a list of changes to the Governed Map parameters.
	///
	/// This function should return data that matches the number of changes and size of data
	/// expected for during normal operation of the benchmarked chain.
	fn changes() -> crate::Changes<T>;

	/// Returns new value of the main chain scripts stored in the pallet.
	///
	/// Because the data size is constant, this function doesn't typically need overriding.
	fn main_chain_scripts() -> crate::MainChainScriptsV1 {
		MainChainScriptsV1::default()
	}
}

impl<T: crate::Config> BenchmarkHelper<T> for () {
	fn changes() -> crate::Changes<T> {
		BoundedVec::truncate_from(alloc::vec![
			(BoundedString::try_from("key1").unwrap(), None),
			(
				BoundedString::try_from("key2").unwrap(),
				Some(BoundedVec::truncate_from(alloc::vec!(1; 128)))
			),
			(
				BoundedString::try_from("key3").unwrap(),
				Some(BoundedVec::truncate_from(alloc::vec!(1; 128)))
			),
			(
				BoundedString::try_from("key4").unwrap(),
				Some(BoundedVec::truncate_from(alloc::vec!(1; 128)))
			),
		])
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register_changes() {
		let changes = T::BenchmarkHelper::changes();

		#[extrinsic_call]
		_(RawOrigin::None, changes);
	}

	#[benchmark]
	fn set_main_chain_scripts() {
		let new_main_chain_scripts = T::BenchmarkHelper::main_chain_scripts();

		#[extrinsic_call]
		_(RawOrigin::Root, new_main_chain_scripts);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
