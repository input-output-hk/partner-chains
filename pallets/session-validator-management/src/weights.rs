
//! Autogenerated weights for pallet_session_validator_management
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-11-15, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `Karans-MacBook-Air.local`, CPU: `<UNKNOWN>`
//! EXECUTION: , WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/debug/partner-chains-node
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_session_validator_management
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./pallets/session-validator-management/src/weights.rs
// --template=./.maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for pallet_session_validator_management.
pub trait WeightInfo {
	fn set(v: u32, ) -> Weight;
}

/// Weights for pallet_session_validator_management using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `Aura::CurrentSlot` (r:1 w:0)
	/// Proof: `Aura::CurrentSlot` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `SessionCommitteeManagement::Committee` (r:1 w:1)
	/// Proof: `SessionCommitteeManagement::Committee` (`max_values`: None, `max_size`: Some(1073), added: 3548, mode: `MaxEncodedLen`)
	/// The range of component `v` is `[0, 32]`.
	fn set(v: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `274`
		//  Estimated: `4538`
		// Minimum execution time: 173_000_000 picoseconds.
		Weight::from_parts(180_296_622, 4538)
			// Standard Error: 128_626
			.saturating_add(Weight::from_parts(1_617_743, 0).saturating_mul(v.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: `Aura::CurrentSlot` (r:1 w:0)
	/// Proof: `Aura::CurrentSlot` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `SessionCommitteeManagement::Committee` (r:1 w:1)
	/// Proof: `SessionCommitteeManagement::Committee` (`max_values`: None, `max_size`: Some(1073), added: 3548, mode: `MaxEncodedLen`)
	/// The range of component `v` is `[0, 32]`.
	fn set(v: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `274`
		//  Estimated: `4538`
		// Minimum execution time: 173_000_000 picoseconds.
		Weight::from_parts(180_296_622, 4538)
			// Standard Error: 128_626
			.saturating_add(Weight::from_parts(1_617_743, 0).saturating_mul(v.into()))
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}