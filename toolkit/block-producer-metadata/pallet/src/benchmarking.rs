//! Benchmarking setup for pallet-address-associations

// TODO needs fixing

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use hex_literal::hex;
use sidechain_domain::*;

#[cfg(test)]
use crate::Pallet as BlockProducerMetadata;

#[benchmarks(where <T as Config>::BlockProducerMetadata: From<crate::mock::BlockProducerUrlMetadata> )]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn upsert_metadata() {
		let metadata = crate::mock::BlockProducerUrlMetadata {
			url: "https://cool.stuff/spo.json".as_bytes().to_vec().try_into().unwrap(),
			hash: hex!("1aa8c1b363a207ddadf0c6242a0632f5a557690a327d0245f9d473b983b3d8e1").into(),
		};
		let cross_chain_pub_key = CrossChainPublicKey(
			hex!("2bebcb7fbc74a6e0fd6e00a311698b047b7b659f0e047ff5349dbd984aefc52c").to_vec(),
		);
		let cross_chain_signature = CrossChainSignature(hex!("1aa8c1b363a207ddadf0c6242a0632f5a557690a327d0245f9d473b983b3d8e1c95a3dd804cab41123c36ddbcb7137b8261c35d5c8ef04ce9d0f8d5c4b3ca607").to_vec());

		#[extrinsic_call]
		_(RawOrigin::None, metadata.into(), cross_chain_signature, cross_chain_pub_key);
	}

	impl_benchmark_test_suite!(
		BlockProducerMetadata,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}
