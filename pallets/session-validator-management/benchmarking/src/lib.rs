#![cfg(any(feature = "runtime-benchmarks", test))]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

use frame_benchmarking::v2::*;
use frame_support::BoundedVec;
use frame_system::Pallet as System;
use frame_system::RawOrigin;
use pallet_aura::Pallet as Aura;
use pallet_session_validator_management::Pallet as SessionCommitteeManagement;
use parity_scale_codec::Encode;
use sp_core::Get;
use sp_runtime::traits::One;
use sp_runtime::{Digest, DigestItem};
use sp_std::{vec, vec::Vec};

const SEED: u32 = 0;
pub const SLOTS_PER_EPOCH: u32 = 60;

pub trait Config: pallet_session_validator_management::Config + pallet_aura::Config {}
pub struct Pallet<T: Config>(SessionCommitteeManagement<T>);

fn set_epoch_number<T: Config>(epoch: u64) {
	let slot = epoch * (SLOTS_PER_EPOCH as u64);

	let pre_runtime_digest = Digest {
		logs: vec![DigestItem::PreRuntime(sp_consensus_aura::AURA_ENGINE_ID, slot.encode())],
	};
	let block_number = System::<T>::block_number() + One::one();
	System::<T>::initialize(&block_number, &System::<T>::parent_hash(), &pre_runtime_digest);
	use frame_support::traits::OnInitialize;
	Aura::<T>::on_initialize(block_number);

	assert_eq!(T::current_epoch_number(), epoch.into());
}

use pallet_session_validator_management::Call;
#[benchmarks]
pub mod benchmarks {
	use super::*;

	/// Benchmarking the `set()` extrinsic
	///
	/// `v` represents the number of validators that will be set in the new validators set
	#[benchmark]
	fn set(v: Linear<0, { T::MaxValidators::get() }>)
	where
		<T as pallet_session_validator_management::Config>::ScEpochNumber: From<ScEpochNumber>,
	{
		let validators: Vec<(
			<T as pallet_session_validator_management::Config>::AuthorityId,
			T::AuthorityKeys,
		)> = (0..v)
			.map(|i| {
				(
					account::<<T as pallet_session_validator_management::Config>::AuthorityId>(
						"member", i, SEED,
					),
					// Contrary to its name, `account` can fill any `Decode`able type with garbage data
					account::<<T as pallet_session_validator_management::Config>::AuthorityKeys>(
						"member", i, SEED,
					),
				)
			})
			.collect();
		let validators: BoundedVec<
			(<T as pallet_session_validator_management::Config>::AuthorityId, T::AuthorityKeys),
			T::MaxValidators,
		> = validators.try_into().unwrap();

		let for_epoch_number = T::current_epoch_number() + One::one();
		set_epoch_number::<T>(for_epoch_number.into());

		#[extrinsic_call]
		_(RawOrigin::None, validators.clone(), for_epoch_number);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
