//! Benchmarking setup for pallet-address-associations

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sidechain_domain::*;

pub trait BenchmarkHelper<BlockProducerUrlMetadata> {
	fn metadata() -> BlockProducerUrlMetadata;
	fn cross_chain_pub_key() -> CrossChainPublicKey;
	fn cross_chain_signature() -> CrossChainSignature;
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn upsert_metadata() {
		let metadata = T::BenchmarkHelper::metadata();
		let cross_chain_pub_key = T::BenchmarkHelper::cross_chain_pub_key();
		let cross_chain_signature = T::BenchmarkHelper::cross_chain_signature();

		#[extrinsic_call]
		_(RawOrigin::None, metadata, cross_chain_signature, cross_chain_pub_key);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
