use crate::*;
use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use sidechain_domain::byte_string::SizedByteString;
use sp_core::Get;
use sp_runtime::traits::One;
use sp_session_validator_management::MainChainScripts;
use sp_std::vec::Vec;

const SEED: u32 = 0;

/// Helper for creating mock data used by benchmarks.
///
/// Only functions without default implementation need to be implemented by chain builders
/// based on their runtime types. Runtimes with `CommitteeMember` types isomorphic to
/// `(AuthorityId, AuthorityKeys)` can use the implementation provided for [()].
pub trait BenchmarkHelper<T: Config> {
	/// Should return `number` of committee members
	fn create_validators(number: u32) -> Vec<T::CommitteeMember>;

	/// Should return an input hash of 32 bytes
	fn create_inputs_hash() -> SizedByteString<32> {
		Default::default()
	}

	fn create_main_chain_scripts() -> MainChainScripts {
		MainChainScripts::default()
	}
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

		let inputs_hash = T::BenchmarkHelper::create_inputs_hash();

		let for_epoch_number = T::current_epoch_number() + One::one();

		#[extrinsic_call]
		_(RawOrigin::None, validators, for_epoch_number, inputs_hash);
	}

	#[benchmark]
	fn set_main_chain_scripts() {
		let MainChainScripts {
			committee_candidate_address,
			d_parameter_policy_id,
			permissioned_candidates_policy_id,
		} = T::BenchmarkHelper::create_main_chain_scripts();

		#[extrinsic_call]
		_(
			RawOrigin::Root,
			committee_candidate_address,
			d_parameter_policy_id,
			permissioned_candidates_policy_id,
		);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
