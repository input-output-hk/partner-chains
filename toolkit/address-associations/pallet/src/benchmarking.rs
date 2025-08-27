//! Benchmarking setup for pallet-address-associations

#![cfg(feature = "runtime-benchmarks")]

// Change to trigger CI2
use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use hex_literal::hex;
use sidechain_domain::*;
use sp_core::crypto::Ss58Codec;

#[cfg(test)]
use crate::Pallet as AddressAssociations;

#[benchmarks(where <T as Config>::PartnerChainAddress: Ss58Codec)]
mod benchmarks {
	use super::*;
	use frame_support::traits::{Get, tokens::fungible::Mutate};

	#[benchmark]
	fn associate_address() {
		// Alice
		let stake_public_key = StakePublicKey(hex!(
			"2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c"
		));
		// Alice (hex: d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d)
		let pc_address = T::PartnerChainAddress::from_ss58check(
			"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
		)
		.unwrap();
		let signature = StakeKeySignature(hex!(
			"36aabd5267699b01c01fb6892f9888ab82a0b853a21dcd863b8241c3049d85163ddf350cbbc8ba724abe7b22d5ae03a7b1429f4cc37fb11afcce041fac1cdd05"
		));

		// Create an account and fund it with sufficient balance
		let caller: T::AccountId = account("caller", 0, 0);
		let _ = T::Currency::mint_into(&caller, T::BurnAmount::get() * 2u32.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), pc_address, signature, stake_public_key);
	}

	impl_benchmark_test_suite!(AddressAssociations, crate::mock::new_test_ext(), crate::mock::Test);
}
