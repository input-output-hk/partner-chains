//! Benchmarking setup for pallet-address-associations

#![cfg(feature = "runtime-benchmarks")]

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

	#[benchmark]
	fn associate_address() {
		// Alice
		let mc_pub_key = MainchainPublicKey(hex!(
			"2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c"
		));
		// Alice
		let pc_address = T::PartnerChainAddress::from_ss58check(
			"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
		)
		.unwrap();
		let mc_signature = hex!("1aa8c1b363a207ddadf0c6242a0632f5a557690a327d0245f9d473b983b3d8e1c95a3dd804cab41123c36ddbcb7137b8261c35d5c8ef04ce9d0f8d5c4b3ca607");

		#[extrinsic_call]
		_(RawOrigin::None, pc_address, mc_signature, mc_pub_key);
	}

	impl_benchmark_test_suite!(AddressAssociations, crate::mock::new_test_ext(), crate::mock::Test);
}
