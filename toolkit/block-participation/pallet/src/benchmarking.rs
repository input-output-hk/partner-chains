#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as BlockParticipation;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks(where <T as crate::Config>::Moment: From<u64>)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn note_processing() -> Result<(), BenchmarkError> {
		#[block]
		{
			BlockParticipation::<T>::note_processing(RawOrigin::None.into(), 42u64.into())?;
		}
		Ok(())
	}

	impl_benchmark_test_suite!(BlockParticipation, crate::mock::new_test_ext(), crate::mock::Test);
}
