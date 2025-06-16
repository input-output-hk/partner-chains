//! Benchmarking setup for pallet-address-associations
//!
//! ## Running benchmarks
//!
//! To benchmark this pallet, the PC Builder should define a `BenchmarkHelper` type implementing the
//! [BenchmarkHelper] type to provide concrete mock values for all generic types used by the pallet:
//!
//! ```rust
//! use sidechain_domain::byte_string::*;
//! use sidechain_domain::{ CrossChainSignature, CrossChainPublicKey };
//! use sp_core::ConstU32;
//! use hex_literal::hex;
//!
//! struct BlockProducerMetadata {
//!     pub url: BoundedString<ConstU32<512>>,
//!     pub hash: SizedByteString<32>,
//! }
//!
//! struct ExampleBenchmarkHelper;
//!
//! impl pallet_block_producer_metadata::benchmarking::BenchmarkHelper<BlockProducerMetadata> for ExampleBenchmarkHelper {
//!     fn metadata() -> BlockProducerMetadata {
//!         BlockProducerMetadata {
//!        	   url: BoundedString::try_from("https://cool.stuff/spo.json").unwrap(),
//!        	   hash: SizedByteString::from([0; 32]),
//!         }
//!     }
//!     fn cross_chain_pub_key() -> CrossChainPublicKey {
//!         CrossChainPublicKey(hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec())
//!     }
//!     fn cross_chain_signature() -> sidechain_domain::CrossChainSignature {
//!         CrossChainSignature(hex!("d1e02e4a5484c3b7202ce6b844577048e7578dc62901cf8f51e6d74bbd3adb091688feacedd8343d0b04a0f5862b2e06148934a75e678e42051fde5431eca33d").to_vec())
//!     }
//! }
//! ```
//!
//! Assuming that the runtime crate uses the feature flag `runtime-benchmarks` to enable benchmarking features,
//! this helper should be then added to the pallet's config:
//! ```rust
//! # struct ExampleBenchmarkHelper;
//! #[cfg(feature = "runtime-benchmarks")]
//! type BenchmarkHelper = ExampleBenchmarkHelper;
//! ```
//! and the pallet's own `runtime-benchmarks` feature should be enabled and the pallet should be included in the
//! runtime's benchmarks list:
//! ```rust, ignore
//! define_benchmarks!(
//!     ...,
//!        [pallet_block_producer_metadata, BlockProducerMetadata]
//! )
//! ```
//!
//! Afterwards, the pallet can be benchmarked using Polkadot SDK's [omini-bencher](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/utils/frame/omni-bencher).

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sidechain_domain::*;

/// Helper trait for injecting mock values for use in benchmarks
pub trait BenchmarkHelper<BlockProducerMetadata> {
	/// Should return mock metadata
	fn metadata() -> BlockProducerMetadata;
	/// Should return mock cross-chain pubkey
	fn cross_chain_pub_key() -> CrossChainPublicKey;
	/// Should return mock cross-chain signature
	///
	/// This signature must match the cross-chain pubkey returned by `cross_chain_pub_key` and be a valid
	/// signature of [MetadataSignedMessage] created using values returned by `metadata` and `cross_chain_pub_key`
	/// and the genesis UTXO used for benchmarks.
	fn cross_chain_signature() -> CrossChainSignature;
}

#[benchmarks]
mod benchmarks {
	use super::*;
	use frame_support::traits::{Currency, Get};

	#[benchmark]
	fn upsert_metadata() {
		let metadata = T::BenchmarkHelper::metadata();
		let cross_chain_pub_key = T::BenchmarkHelper::cross_chain_pub_key();
		let cross_chain_signature = T::BenchmarkHelper::cross_chain_signature();

		// Create an account with sufficient balance for burning
		let caller: T::AccountId = account("caller", 0, 0);
		let burn_amount = T::BurnAmount::get();
		let _ = T::Currency::make_free_balance_be(&caller, burn_amount * 2u32.into());

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), metadata, cross_chain_signature, cross_chain_pub_key);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
