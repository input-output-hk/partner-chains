use crate::*;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use sp_core::Get;
use sp_runtime::traits::One;
use sp_std::vec::Vec;

const SEED: u32 = 0;

/// Helper for creating mock data used by benchmarks.
///
/// Runtimes with `CommitteeMember` types isomorphic to `(AuthorityId, AuthorityKeys)`
/// can use the implementation provided for [()].
pub trait BenchmarkHelper<T: Config> {
	/// Should return `number` of committee members
	fn create_validators(number: u32) -> Vec<T::CommitteeMember>;
}

impl<T: Config> BenchmarkHelper<T> for ()
where
	<T as Config>::CommitteeMember:
		From<(<T as Config>::AuthorityId, <T as Config>::AuthorityKeys)>,
{
	fn create_validators(number: u32) -> Vec<T::CommitteeMember> {
		(0..number)
			.map(|i| {
				let authority = account::<<T as crate::Config>::AuthorityId>("member", i, SEED);
				// Contrary to its name, `account` can fill any `Decode`able type with garbage data
				let keys = account::<<T as crate::Config>::AuthorityKeys>("member", i, SEED);
				From::from((authority, keys))
			})
			.collect()
	}
}

#[benchmarks]
pub mod benchmarks {
	use super::*;

	/// Benchmarking the `set()` extrinsic
	///
	/// `v` represents the number of validators that will be set in the new validators set
	#[benchmark]
	fn set(v: Linear<0, { T::MaxValidators::get() }>)
	where
		<T as crate::Config>::ScEpochNumber: From<ScEpochNumber>,
	{
		let validators: BoundedVec<<T as crate::Config>::CommitteeMember, T::MaxValidators> =
			BoundedVec::truncate_from(T::BenchmarkHelper::create_validators(v));

		let for_epoch_number = T::current_epoch_number() + One::one();

		#[extrinsic_call]
		_(RawOrigin::None, validators, for_epoch_number, Default::default());
	}

	#[benchmark]
	fn set_main_chain_scripts() {
		#[extrinsic_call]
		_(RawOrigin::Root, Default::default(), Default::default(), Default::default());
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
