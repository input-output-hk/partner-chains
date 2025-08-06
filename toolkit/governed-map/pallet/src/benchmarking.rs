//! Benchmarking setup for pallet-governed-map
//!
//! # Running your own benchmarks
//!
//! To run your own benchmarks, you need to implement the [BenchmarkHelper] trait to supply
//! realistic keys and values, eg:
//! ```rust,ignore
//! # use frame_support::testing_prelude::bounded_vec;
//! # use sidechain_domain::bounded_str;
//! # use sidechain_domain::byte_string::BoundedString;
//! struct BenchmarkHelper;
//!
//! impl<T: pallet_governed_map::Config> pallet_governed_map::benchmarking::BenchmarkHelper<T> for BenchmarkHelper {
//! 	fn key(index: u32) -> pallet_governed_map::MapKey<T> {
//! 		bounded_str!("mock key {index}")
//! 	}
//!
//! 	fn value(index: u32) -> Option<pallet_governed_map::MapValue<T>> {
//! 		Some(bounded_vec![index as u8; 128])
//! 	}
//! }
//! ```
//!
//! Once implemented, wire it in your pallet's configuration;
//! ```rust,ignore
//! impl pallet_governed_map::Config for Runtime {
//!     /* ... */
//!     #[cfg(feature = "runtime-benchmarks")]
//!     type BenchmarkHelper = BenchmarkHelper;
//! }
//! ```
//! and include the pallet in your runtime's benchmark list:
//! ```rust, ignore
//! define_benchmarks!(
//!     ...,
//!        [pallet_governed_map, GovernedMap]
//! )
//! ```
//!
//! After this is done, the pallet can be benchmarked using
//! [omini-bencher](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/utils/frame/omni-bencher)
//! from Polkadot SDK.

use super::*;
use alloc::collections::BTreeMap;
use alloc::format;
use frame_benchmarking::v2::*;
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sidechain_domain::bounded_str;

/// Trait for injecting chain-specific test values for benchmarking.
pub trait BenchmarkHelper<T: crate::Config> {
	/// Returns a list of changes to the Governed Map parameters of length `length`.
	fn changes(length: u32) -> crate::Changes<T> {
		BoundedBTreeMap::try_from(
			(0..length).map(|i| (Self::key(i), Self::value(i))).collect::<BTreeMap<_, _>>(),
		)
		.unwrap()
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
		bounded_str!("key{index}")
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
		assert_ok!(Pallet::<T>::set_main_chain_scripts(
			RawOrigin::Root.into(),
			T::BenchmarkHelper::main_chain_scripts(),
		));

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
