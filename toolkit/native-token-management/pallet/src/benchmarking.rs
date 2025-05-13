//! Benchmarking setup for pallet-native-token-management

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as NativeTokenManagement;

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
	use super::*;

	// Benchmark `transfer_tokens` extrinsic with the worst possible conditions:
	// * Successfull operation is the most pesimistic
	#[benchmark]
	fn transfer_tokens() -> Result<(), BenchmarkError> {
		#[block]
		{
			NativeTokenManagement::<T>::transfer_tokens(
				RawOrigin::None.into(),
				NativeTokenAmount::default(),
			)?;
		}

		Ok(())
	}

	// Benchmark `set_main_chain_scripts` extrinsic with the worst possible conditions:
	// * Successfull operation is the most pesimistic
	#[benchmark]
	fn set_main_chain_scripts() -> Result<(), BenchmarkError> {
		#[block]
		{
			NativeTokenManagement::<T>::set_main_chain_scripts(
				RawOrigin::Root.into(),
				PolicyId::default(),
				AssetName::default(),
				MainchainAddress::default(),
			)?;
		}

		Ok(())
	}

	// Benchmark `on_finalize` extrinsic with the worst possible conditions:
	#[benchmark]
	fn on_finalize() -> Result<(), BenchmarkError> {
		NativeTokenManagement::<T>::set_main_chain_scripts(
			RawOrigin::Root.into(),
			PolicyId::default(),
			AssetName::default(),
			MainchainAddress::default(),
		)?;

		NativeTokenManagement::<T>::transfer_tokens(
			RawOrigin::None.into(),
			NativeTokenAmount::default(),
		)?;

		#[block]
		{
			NativeTokenManagement::<T>::on_finalize(frame_system::Pallet::<T>::block_number());
		}

		Ok(())
	}

	impl_benchmark_test_suite!(
		NativeTokenManagement,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}
