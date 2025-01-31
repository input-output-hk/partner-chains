//! Benchmarking setup for pallet-address-associations

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use core::str::FromStr;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use hex_literal::hex;
use sidechain_domain::*;

#[benchmarks(where <T as Config>::PartnerChainAddress: From<[u8; 32]>)]
mod benchmarks {
	use super::*;

	// Benchmark `associate_address` extrinsic with the worst possible conditions:
	// * Successfull operation is the most pesimistic
	#[benchmark]
	fn associate_address() {
		let mc_address = MainchainAddress::from_str(
			"addr_test1vr5vxqpnpl3325cu4zw55tnapjqzzx78pdrnk8k5j7wl72c6y08nd",
		)
		.unwrap();
		let mc_pub_key = MainchainPublicKey(hex!(
			"fc014cb5f071f5d6a36cb5a7e5f168c86555989445a23d4abec33d280f71aca4"
		));
		let mc_signarture = MainchainSignature(hex!("b50828c31d1a61e05fdb943847efd42ce2eadda9c7d21dd2d035e8de66bc56de7f6b1297fba6cb7305f2aac97b5f9168894fb10295c503de6d5fb6ae70bd9a0d"));

		#[extrinsic_call]
		_(RawOrigin::None, mc_address, [0u8; 32].into(), mc_signarture, mc_pub_key);
	}

	impl_benchmark_test_suite!(AddressAssociations, crate::mock::new_test_ext(), crate::mock::Test);
}
