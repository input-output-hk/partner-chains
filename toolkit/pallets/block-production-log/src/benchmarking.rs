//! Benchmarking setup for pallet-block-production-log

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as BlockProductionLog;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_consensus_slots::Slot;
use sp_std::vec::Vec;

#[benchmarks(where <T as Config>::BlockProducerId: From<[u8; 32]>)]
mod benchmarks {

	use super::*;

	fn make_id<T: Config>(i: u64) -> T::BlockProducerId
	where
		<T as Config>::BlockProducerId: From<[u8; 32]>,
	{
		let mut id = [0u8; 32];
		id[0..8].copy_from_slice(&i.to_le_bytes());
		id.into()
	}

	fn setup_storage<T: Config>(num_items: u64)
	where
		<T as Config>::BlockProducerId: From<[u8; 32]>,
	{
		let vec = (0..num_items)
			.into_iter()
			.map(|i| (Slot::from(i), make_id::<T>(i)))
			.collect::<Vec<_>>();
		Log::<T>::put(vec);
	}

	#[benchmark]
	fn append() -> Result<(), BenchmarkError> {
		setup_storage::<T>(59);
		let id = make_id::<T>(1001);
		#[block]
		{
			BlockProductionLog::<T>::append(RawOrigin::None.into(), id)?;
		}
		Ok(())
	}

	impl_benchmark_test_suite!(BlockProductionLog, crate::mock::new_test_ext(), crate::mock::Test);
}
