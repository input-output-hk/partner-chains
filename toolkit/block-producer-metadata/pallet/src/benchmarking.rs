//! Benchmarking setup for pallet-address-associations
//!
//! ## Running benchmarks
//!
//! To benchmark this pallet, the PC Builder should define a `BenchmarkHelper` type implementing the
//! [BenchmarkHelper] type to provide concrete mock values for all generic types used by the pallet:
//!
//! ```rust
//! use sidechain_domain::byte_string::*;
//! use sidechain_domain::{ CrossChainSignature, CrossChainPublicKey, UtxoId };
//! use sp_core::{ ConstU32, Encode };
//! use hex_literal::hex;
//! use sp_runtime::AccountId32;
//!
//! #[derive(Encode)]
//! struct BlockProducerMetadata {
//!     pub url: BoundedString<ConstU32<512>>,
//!     pub hash: SizedByteString<32>,
//! }
//!
//! struct ExampleBenchmarkHelper;
//!
//! impl pallet_block_producer_metadata::benchmarking::BenchmarkHelper<BlockProducerMetadata, AccountId32> for ExampleBenchmarkHelper {
//!     fn genesis_utxo() -> UtxoId {
//!         UtxoId::new([1;32], 0)
//!     }
//!
//!     fn metadata() -> BlockProducerMetadata {
//!         BlockProducerMetadata {
//!        	   url: BoundedString::try_from("https://cool.stuff/spo.json").unwrap(),
//!        	   hash: SizedByteString::from([0; 32]),
//!         }
//!     }
//!     fn cross_chain_pub_key() -> CrossChainPublicKey {
//!         CrossChainPublicKey(hex!("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").to_vec())
//!     }
//!     fn cross_chain_sign_key() -> k256::SecretKey {
//!         k256::SecretKey::from_slice(&[2;32]).unwrap()
//!     }
//!     fn upsert_valid_before() -> u64 {
//!     	100_000_000
//!     }
//!     fn delete_valid_before() -> u64 {
//!     	100_000_000
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

#![cfg(any(feature = "runtime-benchmarks", doc))]
use super::*;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
pub use k256::SecretKey;
use sidechain_domain::*;

/// Helper trait for injecting mock values for use in benchmarks
pub trait BenchmarkHelper<BlockProducerMetadata: Encode, AccountId: Encode> {
	/// Should return the chain's genesis utxo
	fn genesis_utxo() -> UtxoId;
	/// Should return mock metadata
	fn metadata() -> BlockProducerMetadata;
	/// Should return mock cross-chain pubkey
	fn cross_chain_pub_key() -> CrossChainPublicKey;
	/// Should return mock cross-chain signing key
	fn cross_chain_sign_key() -> SecretKey;
	/// Should return mock cross-chain signature for upsert operation
	///
	/// This signature must match the cross-chain pubkey returned by `cross_chain_pub_key` and be a valid
	/// signature of [MetadataSignedMessage] created using values returned by `metadata` and `cross_chain_pub_key`
	/// and the genesis UTXO used for benchmarks.
	fn upsert_cross_chain_signature(owner: AccountId) -> CrossChainSignature {
		MetadataSignedMessage {
			cross_chain_pub_key: Self::cross_chain_pub_key(),
			metadata: Some(Self::metadata()),
			genesis_utxo: Self::genesis_utxo(),
			valid_before: Self::upsert_valid_before(),
			owner,
		}
		.sign_with_key(&Self::cross_chain_sign_key())
	}

	/// Should return mock cross-chain signature for delete operation
	///
	/// This signature must match the cross-chain pubkey returned by `cross_chain_pub_key` and be a valid
	/// for the genesis UTXO used for benchmarks.
	fn delete_cross_chain_signature(owner: AccountId) -> CrossChainSignature {
		MetadataSignedMessage {
			cross_chain_pub_key: Self::cross_chain_pub_key(),
			metadata: None::<BlockProducerMetadata>,
			genesis_utxo: Self::genesis_utxo(),
			valid_before: Self::delete_valid_before(),
			owner,
		}
		.sign_with_key(&Self::cross_chain_sign_key())
	}

	/// Should return the valid-before value for the upsert
	fn upsert_valid_before() -> u64;

	/// Should return the valid-before value for the delete
	fn delete_valid_before() -> u64;
}

#[benchmarks(where <T as Config>::Currency: frame_support::traits::tokens::fungible::Mutate<<T as frame_system::Config>::AccountId>)]
mod benchmarks {
	use super::*;
	use frame_support::traits::fungible::MutateHold;
	use frame_support::traits::{Get, tokens::fungible::Mutate};

	#[benchmark]
	fn upsert_metadata() {
		let metadata = T::BenchmarkHelper::metadata();
		let cross_chain_pub_key = T::BenchmarkHelper::cross_chain_pub_key();
		let valid_before = T::BenchmarkHelper::upsert_valid_before();

		// Create an account and fund it with sufficient balance
		let caller: T::AccountId = account("caller", 0, 0);
		let _ = T::Currency::mint_into(&caller, T::HoldAmount::get() * 2u32.into());

		let cross_chain_signature =
			T::BenchmarkHelper::upsert_cross_chain_signature(caller.clone());

		#[extrinsic_call]
		_(
			RawOrigin::Signed(caller),
			metadata,
			cross_chain_signature,
			cross_chain_pub_key,
			valid_before,
		);
	}

	#[benchmark]
	fn delete_metadata() {
		let metadata = T::BenchmarkHelper::metadata();
		let cross_chain_pub_key = T::BenchmarkHelper::cross_chain_pub_key();
		let valid_before = T::BenchmarkHelper::delete_valid_before();

		let caller: T::AccountId = account("caller", 0, 0);
		let _ =
			T::Currency::hold(&HoldReason::MetadataDeposit.into(), &caller, T::HoldAmount::get());

		let cross_chain_signature =
			T::BenchmarkHelper::delete_cross_chain_signature(caller.clone());

		BlockProducerMetadataStorage::<T>::insert(
			cross_chain_pub_key.hash(),
			(metadata, caller.clone(), T::HoldAmount::get()),
		);

		#[extrinsic_call]
		_(RawOrigin::Signed(caller), cross_chain_pub_key, cross_chain_signature, valid_before);
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
