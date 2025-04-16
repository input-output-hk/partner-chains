//! Benchmarking setup for pallet-governed-map

use super::*;
use alloc::format;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

/// Trait for injecting chain-specific test values for benchmarking.
pub trait BenchmarkHelper<T: crate::Config> {
	/// Returns a list of changes to the Governed Map parameters of length `length`.
	fn changes(length: u32) -> crate::Changes<T> {
		BoundedVec::truncate_from((0..length).map(|i| (Self::key(i), Self::value(i))).collect())
	}

	/// Returns `index`th mock Governance Map key
	///
	/// Size of data returned by this function should match the expected distribution
	fn key(index: u32) -> crate::MapKey<T>;

	/// Returns `index`th mock Governance Map value
	///
	/// Size of data returned by this function should match the expected distribution
	fn value(index: u32) -> Option<crate::MapValue<T>>;

	/// Returns new value of the main chain scripts stored in the pallet.
	///
	/// Because the data size is constant, this function doesn't typically need overriding.
	fn main_chain_scripts() -> crate::MainChainScriptsV1 {
		MainChainScriptsV1::default()
	}
}

impl<T: crate::Config> BenchmarkHelper<T> for () {
	fn key(index: u32) -> crate::MapKey<T> {
		BoundedString::try_from(format!("key{index}").as_str()).unwrap()
	}

	fn value(index: u32) -> Option<crate::MapValue<T>> {
		let i = (index % (u8::MAX as u32)) as u8;
		Some(BoundedVec::truncate_from(alloc::vec!(i; 128)))
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn register_changes(l: Linear<1, { T::MaxChanges::get() }>) {
		let changes = T::BenchmarkHelper::changes(l);

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
