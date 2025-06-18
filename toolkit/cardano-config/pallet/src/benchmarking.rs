//! Benchmarking setup for pallet-cardano-config

#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v2::*;
use frame_support::{assert_ok, traits::Get};
use sidechain_domain::{CardanoConfig, MainchainEpochConfig};
use sp_core::offchain::{Duration, Timestamp};

/// Helper trait for benchmarking
pub trait BenchmarkHelper<T: Config> {
	/// Returns a sample CardanoConfig for benchmarking
	fn cardano_config() -> CardanoConfig;
}

impl<T: Config> BenchmarkHelper<T> for () {
	fn cardano_config() -> CardanoConfig {
		CardanoConfig {
			epoch_config: MainchainEpochConfig {
				epoch_duration_millis: Duration::from_millis(432000000), // 5 days
				slot_duration_millis: Duration::from_millis(1000),       // 1 second
				first_epoch_timestamp_millis: Timestamp::from_unix_millis(1596059091000),
				first_epoch_number: 208,
				first_slot_number: 4492800,
			},
			cardano_security_parameter: 432,
			cardano_active_slots_coeff: 0.05,
		}
	}
}

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn set_cardano_config() {
		let config = T::BenchmarkHelper::cardano_config();

		#[extrinsic_call]
		_(RawOrigin::Root, config.clone());

		assert_eq!(CardanoConfiguration::<T>::get(), Some(config.clone()));
		assert_eq!(MainchainEpochConfiguration::<T>::get(), Some(config.epoch_config));
		assert_eq!(CardanoSecurityParameter::<T>::get(), Some(config.cardano_security_parameter));
		assert_eq!(CardanoActiveSlotsCoeff::<T>::get(), Some(config.cardano_active_slots_coeff));
	}

	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test);
}
