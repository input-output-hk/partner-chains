//! Benchmarking setup for pallet-block-production-log

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as BlockProductionLog;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_consensus_slots::Slot;
use sp_std::vec::Vec;

/// Trait for injecting chain-specific test values for benchmarking.
pub trait BenchmarkHelper<BlockProducerId> {
	/// Returns block producer id
	fn producer_id() -> BlockProducerId;
}

#[benchmarks]
mod benchmarks {

	use super::*;

	fn setup_storage<T: Config>(num_items: u64) {
		let vec = (0..num_items)
			.into_iter()
			.map(|i| (Slot::from(i), T::BenchmarkHelper::producer_id()))
			.collect::<Vec<_>>();
		Log::<T>::put(vec);
	}

	#[benchmark]
	fn append() -> Result<(), BenchmarkError> {
		setup_storage::<T>(59);
		let id = T::BenchmarkHelper::producer_id();
		#[block]
		{
			BlockProductionLog::<T>::append(RawOrigin::None.into(), id)?;
		}
		Ok(())
	}

	impl_benchmark_test_suite!(BlockProductionLog, crate::mock::new_test_ext(), crate::mock::Test);
}
