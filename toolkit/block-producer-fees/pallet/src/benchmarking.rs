//! Benchmarking setup for pallet-block-producer-fees

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;

/// Trait for injecting chain-specific test values for benchmarking.
pub trait BenchmarkHelper<T> {
	/// Makes account ids for benchmarking purpose
	fn account_id(i: u8) -> T;
}

#[benchmarks]
mod benchmarks {
	use super::*;
	use frame_support::traits::Get;
	use frame_system::RawOrigin;
	use frame_system::pallet_prelude::OriginFor;
	use sp_consensus_slots::Slot;
	use sp_std::collections::vec_deque::VecDeque;

	// Pessimistic storage for the ID
	fn setup_storage<T: Config>() {
		let size = T::HistoricalChangesPerProducer::get();
		// Pessimistic storage content for is full, because it requires additional removal from the vecdeque.
		let data = (0..size).into_iter().map(|_| (Slot::from(0), 0u16)).collect::<VecDeque<_>>();
		for i in 0u8..100u8 {
			let id = T::BenchmarkHelper::account_id(i);
			FeesChanges::<T>::insert(id, data.clone());
		}
	}

	#[benchmark]
	fn set_fee() {
		setup_storage::<T>();
		let origin: OriginFor<T> = RawOrigin::Signed(T::BenchmarkHelper::account_id(42)).into();
		#[extrinsic_call]
		_(origin, 100u16);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
